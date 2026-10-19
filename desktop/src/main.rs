#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use eframe::egui::{self, Color32, RichText};
use gamepad_desktop::runtime::DesktopRuntime;
use gamepad_desktop::tray::{PanelControl, TrayAction, create_tray};
use gamepad_desktop::{
    control::{KEYBOARD_SHORTCUTS, MOUSE_SHORTCUTS},
    keyboard::{KEY_ROWS, KeyboardKey, KeyboardPresentation, KeyboardSession},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

type SharedDesktop = Arc<Mutex<DesktopRuntime>>;

struct GamepadNavigatorApp {
    desktop: SharedDesktop,
    panel: Arc<PanelControl>,
    tray_error: Option<String>,
    mapping_text: String,
}

impl GamepadNavigatorApp {
    fn new(desktop: SharedDesktop, panel: Arc<PanelControl>, tray_error: Option<String>) -> Self {
        Self {
            desktop,
            panel,
            tray_error,
            mapping_text: String::new(),
        }
    }
}

impl eframe::App for GamepadNavigatorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if ui.input(|input| input.viewport().close_requested())
            && self.tray_error.is_none()
            && !self.panel.exiting()
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        let keyboard = self
            .desktop
            .lock()
            .expect("desktop runtime lock")
            .state()
            .keyboard;
        if keyboard.presentation == KeyboardPresentation::Viewport {
            show_keyboard_viewport(ui, keyboard);
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Controle do desktop");
                let mut state = self.desktop.lock().expect("desktop runtime lock").state();
                if let Some(error) = &state.error {
                    ui.colored_label(Color32::LIGHT_RED, error);
                    if ui.button("Tentar novamente").clicked() {
                        let runtime = DesktopRuntime::default();
                        *self.desktop.lock().expect("desktop runtime lock") = runtime;
                    }
                } else if !state.ready {
                    ui.label("Conectando ao sistema…");
                } else if let Some(name) = &state.controller {
                    ui.colored_label(Color32::LIGHT_GREEN, format!("Controle: {name}"));
                    ui.label(if state.keyboard.open {
                        "Teclado visual próprio · direcional move · A digita · L3 fecha"
                    } else if state.settings.enabled {
                        "Modo mouse · solte os botões e centralize os analógicos ao conectar/retomar"
                    } else {
                        "Pausado · pressione Start para retomar"
                    });
                } else {
                    ui.label(if state.devices.is_empty() { "Conecte um gamepad. A detecção é automática." } else { "Selecione um controle com perfil ou importe um perfil abaixo." });
                    ui.small("USB/Bluetooth · Xbox, PlayStation, Switch e perfis SDL para genéricos.");
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if state.keyboard.open { "TECLADO VISUAL" } else if state.settings.enabled { "MOUSE" } else { "PAUSADO" }).strong().size(20.0).color(Color32::LIGHT_GREEN));
                });
                if state.keyboard.presentation != KeyboardPresentation::Unavailable
                    && let Some(status) = &state.keyboard.status
                {
                    ui.small(status);
                }
                if state.keyboard.open {
                    ui.small("O teclado não ativa a própria janela; o campo e o cursor de texto permanecem no aplicativo anterior.");
                }
                if state.keyboard.presentation == KeyboardPresentation::Unavailable
                    && let Some(status) = &state.keyboard.status
                {
                    ui.colored_label(Color32::LIGHT_RED, status);
                }
                ui.separator();
                ui.heading("Comandos do desktop");
                shortcut_guide(ui, MOUSE_SHORTCUTS);
                ui.collapsing("Comandos do teclado visual", |ui| {
                    shortcut_guide(ui, KEYBOARD_SHORTCUTS);
                });
                ui.small("Os nomes inferior, direito, esquerdo e superior indicam a posição física dos quatro botões. Escape cancela menus; não é o comando Voltar do navegador.");
                ui.separator();
                egui::CollapsingHeader::new("Controle e ajustes").show(ui, |ui| {
                    let mut selected = state.selected;
                    let selected_name = if selected.is_none() { "Automático" } else { state.devices.iter().find(|device| Some(device.id) == selected).map_or("Controle desconectado", |device| device.name.as_str()) };
                    egui::ComboBox::from_id_salt("desktop-controller").selected_text(selected_name).show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, None, "Automático");
                        for device in &state.devices {
                            let label = format!("{}{}", device.name, if device.mapped { "" } else { " · precisa de perfil" });
                            ui.selectable_value(&mut selected, Some(device.id), label);
                        }
                    });
                    if selected != state.selected {
                        self.desktop.lock().expect("desktop runtime lock").select_controller(selected);
                    }
                    let mut changed = ui.checkbox(&mut state.settings.enabled, "Controlar desktop (Start pausa/retoma)").changed();
                    changed |= ui.add(egui::Slider::new(&mut state.settings.speed, 100.0..=2400.0).text("Velocidade")).changed();
                    changed |= ui.add(egui::Slider::new(&mut state.settings.deadzone, 0.05..=0.5).text("Zona morta")).changed();
                    if changed {
                        self.desktop.lock().expect("desktop runtime lock").configure(state.settings);
                    }
                });
                if let Some(error) = &self.tray_error {
                    ui.colored_label(
                        Color32::LIGHT_RED,
                        format!("Bandeja indisponível: {error}. Fechar o painel encerra o app."),
                    );
                } else {
                    ui.label(
                        "Fechar o painel mantém o controle ativo na bandeja, perto do relógio.",
                    );
                }
                if ui.button("Encerrar aplicativo").clicked() {
                    self.panel.exit();
                }
                egui::CollapsingHeader::new("Perfis de controles").show(ui, |ui| {
                    ui.label("Controles conhecidos usam o perfil SDL automaticamente. Para um modelo desconhecido, importe uma linha de mapeamento SDL.");
                    for device in &state.devices {
                        ui.label(format!("{} · GUID: {}", device.name, device.guid));
                    }
                    ui.add(egui::TextEdit::multiline(&mut self.mapping_text).hint_text("GUID,nome,a:b0,leftx:a0,lefty:a1,...").desired_rows(2));
                    if ui.add_enabled(!self.mapping_text.trim().is_empty(), egui::Button::new("Aplicar e salvar perfil")).clicked() {
                        self.desktop.lock().expect("desktop runtime lock").import_mapping(self.mapping_text.clone());
                    }
                    if let Some(message) = &state.mapping_message { ui.label(message); }
                });

            });
        });
    }
}

fn show_keyboard_viewport(ui: &mut egui::Ui, session: KeyboardSession) {
    let screen = ui
        .ctx()
        .input(|input| input.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    let size = egui::vec2(1200.0, 380.0);
    let position = egui::pos2(
        (screen.x - size.x).max(0.0) / 2.0,
        (screen.y - size.y - 20.0).max(0.0),
    );
    let viewport = egui::ViewportId::from_hash_of("gamepad-navigator-keyboard");
    ui.ctx().show_viewport_deferred(
        viewport,
        egui::ViewportBuilder::default()
            .with_title("Gamepad Navigator · Teclado")
            .with_inner_size(size)
            .with_position(position)
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_active(false)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_taskbar(false)
            .with_visible(true),
        move |ui, _| keyboard_viewport_ui(ui, &session),
    );
}

fn keyboard_viewport_ui(ui: &mut egui::Ui, session: &KeyboardSession) {
    ui.ctx().request_repaint_after(Duration::from_millis(50));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
        .show(ui, |ui| {
            if !session.open {
                return;
            }
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(15, 20, 31, 248))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(56, 189, 175)))
                .corner_radius(16)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("TECLADO VIRTUAL")
                                .strong()
                                .size(12.0)
                                .color(Color32::from_rgb(83, 225, 204)),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Selecionada: {}",
                                session.selected_key().label(session.shift)
                            ))
                            .strong()
                            .size(18.0)
                            .color(Color32::WHITE),
                        );
                        if session.shift {
                            ui.label(
                                RichText::new("SHIFT ATIVO")
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::from_rgb(255, 218, 120)),
                            );
                        }
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new("D-pad navega  ·  A digita")
                                        .size(13.0)
                                        .color(Color32::from_gray(190)),
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
                        RichText::new(
                            "A digita   ·   B apaga   ·   X Enter   ·   Y Esc   ·   R3 espaço   ·   L1 Tab   ·   R1 Shift   ·   L3 fecha",
                        )
                        .size(13.0)
                        .color(Color32::from_gray(205)),
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
        Color32::from_rgb(13, 148, 136)
    } else if special {
        Color32::from_rgb(58, 72, 91)
    } else {
        Color32::from_rgb(38, 48, 64)
    };
    let label_size = match key {
        KeyboardKey::Character(_) | KeyboardKey::Shift | KeyboardKey::Backspace => 19.0,
        KeyboardKey::Space => 15.0,
        _ => 16.0,
    };
    egui::Button::new(
        RichText::new(key.label(shift))
            .strong()
            .size(label_size)
            .color(Color32::WHITE),
    )
    .fill(fill)
    .stroke(egui::Stroke::new(
        if selected { 3.0 } else { 1.0 },
        if selected {
            Color32::from_rgb(130, 255, 231)
        } else {
            Color32::from_rgb(82, 97, 117)
        },
    ))
    .corner_radius(8)
}

fn shortcut_guide(ui: &mut egui::Ui, shortcuts: &[(&str, &str)]) {
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .striped(true)
        .spacing([24.0, 9.0])
        .show(ui, |ui| {
            for (button, action) in shortcuts {
                ui.label(
                    RichText::new(*button)
                        .strong()
                        .color(Color32::from_rgb(100, 220, 200)),
                );
                ui.label(*action);
                ui.end_row();
            }
        });
}

fn main() -> eframe::Result {
    // The service belongs to the process, so closing a Wayland window keeps it alive.
    let desktop = Arc::new(Mutex::new(DesktopRuntime::default()));
    let (panel, actions) = PanelControl::new();
    let tray = create_tray(Arc::clone(&panel));
    let tray_error = tray.as_ref().err().cloned();
    loop {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Gamepad Navigator")
                .with_inner_size([860.0, 760.0])
                .with_min_inner_size([660.0, 500.0]),
            run_and_return: true,
            ..Default::default()
        };

        let app_desktop = Arc::clone(&desktop);
        let app_panel = Arc::clone(&panel);
        let app_error = tray_error.clone();
        eframe::run_native(
            "Gamepad Navigator",
            options,
            Box::new(move |creation_context| {
                creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
                app_panel.attach(Some(creation_context.egui_ctx.clone()));
                Ok(Box::new(GamepadNavigatorApp::new(
                    app_desktop,
                    app_panel,
                    app_error,
                )))
            }),
        )?;
        panel.attach(None);
        if panel.exiting() || tray.is_err() || cfg!(target_os = "windows") {
            break;
        }
        match actions.recv() {
            Ok(TrayAction::Open) => {}
            Ok(TrayAction::Exit) | Err(_) => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_guides_fit_the_minimum_panel_width() {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(660.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut output = context.run_pass(input, |context| {
            let mut root = egui::Ui::new(
                context.clone(),
                egui::Id::unique("shortcut-guide-test"),
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(660.0, 1000.0),
                )),
            );
            egui::CentralPanel::default().show(&mut root, |ui| {
                shortcut_guide(ui, MOUSE_SHORTCUTS);
                assert!(
                    ui.min_rect().right() <= 660.0,
                    "guide overflows: {:?}",
                    ui.min_rect()
                );
            });
        });
        output.textures_delta.clear();
    }
}
