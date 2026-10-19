#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use eframe::egui::{self, Color32, RichText};
use gamepad_desktop::runtime::DesktopRuntime;
use gamepad_desktop::tray::{PanelControl, TrayAction, create_tray};
use gamepad_desktop::{
    control::{KEYBOARD_SHORTCUTS, MOUSE_SHORTCUTS},
    keyboard::{self, KeyboardPosition, KeyboardPresentation, KeyboardSession},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

mod keyboard_view;

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
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        handle_panel_close(ui.ctx(), &self.panel, self.tray_error.is_none());
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
                egui::CollapsingHeader::new("Personalizar teclado QWERTY").show(ui, |ui| {
                    let mut settings = state.keyboard.settings;
                    let mut changed = ui.add(egui::Slider::new(&mut settings.width, 520..=1000).text("Largura (px)").step_by(20.0)).changed();
                    ui.horizontal(|ui| {
                        changed |= ui.selectable_value(&mut settings.position, KeyboardPosition::Top, "Em cima").changed();
                        changed |= ui.selectable_value(&mut settings.position, KeyboardPosition::Bottom, "Embaixo").changed();
                    });
                    changed |= ui.add(egui::Slider::new(&mut settings.opacity, 160..=255).text("Opacidade")).changed();
                    changed |= ui.checkbox(&mut settings.repeat_enabled, "Repetir direcional ao segurar").changed();
                    ui.add_enabled_ui(settings.repeat_enabled, |ui| {
                        changed |= ui.add(egui::Slider::new(&mut settings.repeat_delay_ms, 400..=1200).text("Espera inicial (ms)").step_by(50.0)).changed();
                        changed |= ui.add(egui::Slider::new(&mut settings.repeat_interval_ms, 150..=1000).text("Intervalo (ms)").step_by(50.0)).changed();
                    });
                    if ui.button("Restaurar ajustes do teclado").clicked() {
                        settings = Default::default();
                        changed = true;
                    }
                    if changed {
                        self.desktop.lock().expect("desktop runtime lock").configure_keyboard(settings);
                    }
                    ui.small("O teclado acompanha o campo quando o aplicativo fornece sua posição. Em cima/Embaixo define a alternativa quando essa informação não está disponível.");
                    ui.small("Com o teclado aberto, Start também abre estes ajustes pelo gamepad. As preferências são salvas ao voltar.");
                    ui.small("QWERTY ANSI · Shift vale para a próxima letra · Caps fixa maiúsculas · Ctrl, Alt e Super combinam com a próxima tecla.");
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

fn handle_panel_close(context: &egui::Context, panel: &PanelControl, tray_available: bool) {
    // Wayland cannot hide a native window. Linux closes the panel and waits in
    // main's tray loop; the desktop worker and layer keyboard belong to the process.
    if cfg!(target_os = "windows")
        && context.input(|input| input.viewport().close_requested())
        && tray_available
        && !panel.exiting()
    {
        context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}

fn show_keyboard_viewport(ui: &mut egui::Ui, session: KeyboardSession) {
    let screen = ui
        .ctx()
        .input(|input| input.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    let (width, height) = session.settings.surface_size();
    let viewport = egui::ViewportId::from_hash_of("gamepad-navigator-keyboard");
    let pixels_per_point = ui
        .ctx()
        .input(|i| {
            i.raw
                .viewports
                .get(&viewport)
                .and_then(|v| v.native_pixels_per_point)
        })
        .unwrap_or_else(|| ui.ctx().native_pixels_per_point().unwrap_or(1.0));
    // UIA uses physical pixels; egui viewport positions use logical points.
    let factor = if cfg!(target_os = "windows") {
        pixels_per_point
    } else {
        1.0
    };
    let screen =
        session
            .focused_input
            .and_then(|f| f.screen)
            .unwrap_or(gamepad_desktop::placement::Rect {
                x: 0,
                y: 0,
                width: (screen.x * factor) as i32,
                height: (screen.y * factor) as i32,
            });
    let rect = gamepad_desktop::placement::keyboard_rect(
        screen,
        session.focused_input.map(|f| f.bounds),
        (
            (width as f32 * factor) as u32,
            (height as f32 * factor) as u32,
        ),
        session.settings.position == KeyboardPosition::Top,
    );
    let size = egui::vec2(rect.width as f32 / factor, rect.height as f32 / factor);
    let position = egui::pos2(rect.x as f32 / factor, rect.y as f32 / factor);
    ui.ctx().show_viewport_deferred(
        viewport,
        keyboard_viewport_builder(size, position, session.open),
        move |ui, _| keyboard_viewport_ui(ui, &session),
    );
}

fn keyboard_viewport_builder(
    size: egui::Vec2,
    position: egui::Pos2,
    open: bool,
) -> egui::ViewportBuilder {
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
        .with_has_shadow(false)
        .with_visible(open)
}

#[cfg(test)]
mod overlay_tests {
    use super::*;
    #[test]
    fn closed_keyboard_has_no_visible_window_or_native_shadow() {
        let builder = keyboard_viewport_builder(egui::vec2(640.0, 229.0), egui::Pos2::ZERO, false);
        assert_eq!(builder.visible, Some(false));
        assert_eq!(builder.has_shadow, Some(false));
        assert_eq!(builder.transparent, Some(true));
        assert_eq!(builder.active, Some(false));
    }
}

fn keyboard_viewport_ui(ui: &mut egui::Ui, session: &KeyboardSession) {
    ui.ctx().request_repaint_after(Duration::from_millis(50));
    keyboard_view::show(ui, session);
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

    #[cfg(target_os = "windows")]
    #[test]
    fn closing_windows_panel_hides_it_without_ending_the_tray_message_loop() {
        let (panel, _) = PanelControl::new();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = egui::Context::default()
            .run_pass(input, |context| handle_panel_close(context, &panel, true));
        output.textures_delta.clear();
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Visible(false)))
        );
        assert!(!panel.exiting());
    }

    #[test]
    fn explicit_exit_is_never_canceled_by_close_to_tray() {
        let (panel, actions) = PanelControl::new();
        panel.exit();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = egui::Context::default()
            .run_pass(input, |context| handle_panel_close(context, &panel, true));
        output.textures_delta.clear();
        assert!(
            !output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert_eq!(actions.try_recv().unwrap(), TrayAction::Exit);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn closing_linux_panel_keeps_the_service_but_allows_the_window_to_close() {
        let (panel, actions) = PanelControl::new();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = egui::Context::default()
            .run_pass(input, |context| handle_panel_close(context, &panel, true));
        output.textures_delta.clear();
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose)),
            "Wayland cannot hide a window with Visible(false); allow its actual close"
        );
        assert!(!panel.exiting());
        assert!(actions.try_recv().is_err());
    }

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
