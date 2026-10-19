use std::os::fd::AsFd;

use egui_overlay as egui;
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, EventQueue, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_seat, wl_shm, wl_surface},
};

use crate::keyboard::{KEY_ROWS, KeyboardKey, KeyboardSession};

const SURFACE_WIDTH: u32 = 1200;
const SURFACE_HEIGHT: u32 = 380;

pub struct WaylandKeyboardOverlay {
    connection: Connection,
    queue: EventQueue<OverlayState>,
    state: OverlayState,
}

impl WaylandKeyboardOverlay {
    pub fn new() -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|error| format!("Wayland indisponível: {error}"))?;
        let (globals, queue) = registry_queue_init(&connection)
            .map_err(|error| format!("Não foi possível consultar o compositor Wayland: {error}"))?;
        let handle = queue.handle();
        let compositor = CompositorState::bind(&globals, &handle)
            .map_err(|error| format!("wl_compositor indisponível: {error}"))?;
        let layer_shell = LayerShell::bind(&globals, &handle)
            .map_err(|_| "Este compositor Wayland não oferece wlr-layer-shell; teclado próprio sem foco indisponível.".to_owned())?;
        let shm = Shm::bind(&globals, &handle)
            .map_err(|error| format!("wl_shm indisponível: {error}"))?;
        let surface = compositor.create_surface(&handle);
        let layer = layer_shell.create_layer_surface(
            &handle,
            surface,
            Layer::Overlay,
            Some("gamepad-navigator-keyboard"),
            None,
        );
        layer.set_anchor(Anchor::BOTTOM);
        layer.set_margin(0, 0, 20, 0);
        layer.set_size(SURFACE_WIDTH, SURFACE_HEIGHT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        let input_region = Region::new(&compositor)
            .map_err(|error| format!("Não foi possível criar a região sem mouse: {error}"))?;
        layer
            .wl_surface()
            .set_input_region(Some(input_region.wl_region()));
        layer.commit();

        let mut state = OverlayState {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &handle),
            output_state: OutputState::new(&globals, &handle),
            shm,
            layer,
            pool: None,
            context: egui::Context::default(),
            renderer: EguiSoftwareRender::new(ColorFieldOrder::Bgra).with_caching(true),
            session: KeyboardSession::default(),
            width: SURFACE_WIDTH,
            height: SURFACE_HEIGHT,
            configured: false,
            dirty: true,
        };
        state.context.set_visuals(egui::Visuals::dark());
        state.pool = Some(
            SlotPool::new((SURFACE_WIDTH * SURFACE_HEIGHT * 4) as usize, &state.shm)
                .map_err(|error| format!("Não foi possível alocar o teclado Wayland: {error}"))?,
        );

        let mut overlay = Self {
            connection,
            queue,
            state,
        };
        overlay
            .queue
            .roundtrip(&mut overlay.state)
            .map_err(|error| format!("O compositor não configurou a janela do teclado: {error}"))?;
        overlay.state.configured = true;
        overlay.state.draw()?;
        Ok(overlay)
    }

    pub fn update(&mut self, session: &KeyboardSession) -> Result<(), String> {
        if self.state.session != *session {
            self.state.session = session.clone();
            self.state.dirty = true;
        }
        self.connection
            .flush()
            .map_err(|error| format!("Falha ao atualizar a camada Wayland: {error}"))?;
        while self
            .queue
            .dispatch_pending(&mut self.state)
            .map_err(|error| format!("Falha ao processar eventos Wayland: {error}"))?
            > 0
        {}

        if let Some(read) = self.queue.prepare_read() {
            let fd = self.connection.as_fd();
            let mut poll_fd = [PollFd::new(&fd, PollFlags::IN)];
            let timeout = Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            if poll(&mut poll_fd, Some(&timeout))
                .map_err(|error| format!("Falha ao aguardar eventos Wayland: {error}"))?
                > 0
            {
                read.read()
                    .map_err(|error| format!("Falha ao ler eventos Wayland: {error}"))?;
            }
        }

        while self
            .queue
            .dispatch_pending(&mut self.state)
            .map_err(|error| format!("Falha ao processar eventos Wayland: {error}"))?
            > 0
        {}
        if self.state.dirty && self.state.configured {
            self.state.draw()?;
        }
        Ok(())
    }
}

struct OverlayState {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    layer: LayerSurface,
    pool: Option<SlotPool>,
    context: egui::Context,
    renderer: EguiSoftwareRender,
    session: KeyboardSession,
    width: u32,
    height: u32,
    configured: bool,
    dirty: bool,
}

impl OverlayState {
    fn draw(&mut self) -> Result<(), String> {
        let width = self.width.max(1);
        let height = self.height.max(1);
        let pool = self.pool.as_mut().expect("Wayland buffer pool initialized");
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                width as i32 * 4,
                wl_shm::Format::Argb8888,
            )
            .map_err(|error| format!("Falha ao criar o buffer do teclado: {error}"))?;

        if self.session.open {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width as f32, height as f32),
                )),
                ..Default::default()
            };
            let output = self.context.run_ui(input, |ui| {
                keyboard_ui(ui, &self.session);
            });
            let paint_jobs = self
                .context
                .tessellate(output.shapes, output.pixels_per_point);
            let pixels = bytemuck::cast_slice_mut(canvas);
            let mut buffer_ref = BufferMutRef::new(pixels, width as usize, height as usize);
            self.renderer.render(
                &mut buffer_ref,
                &paint_jobs,
                &output.textures_delta,
                output.pixels_per_point,
            );
        } else {
            canvas.fill(0);
        }

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        buffer
            .attach_to(self.layer.wl_surface())
            .map_err(|error| format!("Falha ao anexar o teclado à camada: {error}"))?;
        self.layer.commit();
        self.dirty = false;
        Ok(())
    }
}

fn keyboard_ui(ui: &mut egui::Ui, session: &KeyboardSession) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::TRANSPARENT))
        .show_inside(ui, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(15, 20, 31, 248))
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    egui::Color32::from_rgb(56, 189, 175),
                ))
                .corner_radius(16)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("TECLADO VIRTUAL")
                                .strong()
                                .size(12.0)
                                .color(egui::Color32::from_rgb(83, 225, 204)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Selecionada: {}",
                                session.selected_key().label(session.shift)
                            ))
                            .strong()
                            .size(18.0)
                            .color(egui::Color32::WHITE),
                        );
                        if session.shift {
                            ui.label(
                                egui::RichText::new("SHIFT ATIVO")
                                    .strong()
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 218, 120)),
                            );
                        }
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new("D-pad navega  ·  A digita")
                                        .size(13.0)
                                        .color(egui::Color32::from_gray(190)),
                                );
                            },
                        );
                    });
                    ui.separator();

                    let mut selected_index = 0;
                    for row in KEY_ROWS {
                        ui.horizontal_centered(|ui| {
                            for key in *row {
                                let selected = selected_index == session.selected_index();
                                ui.add_sized(
                                    [keyboard_key_width(*key), 48.0],
                                    keyboard_key_button(*key, session.shift, selected),
                                );
                                selected_index += 1;
                            }
                        });
                    }

                    ui.separator();
                    ui.label(
                        egui::RichText::new(
                            "A digita   ·   B apaga   ·   X Enter   ·   Y Esc   ·   R3 espaço   ·   L1 Tab   ·   R1 Shift   ·   L3 fecha",
                        )
                        .size(13.0)
                        .color(egui::Color32::from_gray(205)),
                    );
                });
        });
}

fn keyboard_key_width(key: KeyboardKey) -> f32 {
    match key {
        KeyboardKey::Backspace => 92.0,
        KeyboardKey::Enter | KeyboardKey::Shift => 88.0,
        KeyboardKey::Tab | KeyboardKey::Escape => 76.0,
        KeyboardKey::Space => 360.0,
        KeyboardKey::Character(_) => 58.0,
    }
}

fn keyboard_key_button(key: KeyboardKey, shift: bool, selected: bool) -> egui::Button<'static> {
    let special = !matches!(key, KeyboardKey::Character(_));
    let fill = if selected {
        egui::Color32::from_rgb(13, 148, 136)
    } else if special {
        egui::Color32::from_rgb(58, 72, 91)
    } else {
        egui::Color32::from_rgb(38, 48, 64)
    };
    let label_size = match key {
        KeyboardKey::Character(_) | KeyboardKey::Shift | KeyboardKey::Backspace => 19.0,
        KeyboardKey::Space => 15.0,
        _ => 16.0,
    };
    egui::Button::new(
        egui::RichText::new(key.label(shift))
            .strong()
            .size(label_size)
            .color(egui::Color32::WHITE),
    )
    .fill(fill)
    .stroke(egui::Stroke::new(
        if selected { 3.0_f32 } else { 1.0_f32 },
        if selected {
            egui::Color32::from_rgb(130, 255, 231)
        } else {
            egui::Color32::from_rgb(82, 97, 117)
        },
    ))
    .corner_radius(8)
}

impl CompositorHandler for OverlayState {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
        self.dirty = true;
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
        self.dirty = true;
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for OverlayState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.session.open = false;
        self.dirty = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if configure.new_size.0 > 0 {
            self.width = configure.new_size.0;
        }
        if configure.new_size.1 > 0 {
            self.height = configure.new_size.1;
        }
        self.configured = true;
        self.dirty = true;
    }
}

impl OutputHandler for OverlayState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl SeatHandler for OverlayState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ProvidesRegistryState for OverlayState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

impl ShmHandler for OverlayState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(OverlayState);
delegate_layer!(OverlayState);
delegate_output!(OverlayState);
delegate_registry!(OverlayState);
delegate_seat!(OverlayState);
delegate_shm!(OverlayState);
