#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use eframe::egui::{self, Color32, RichText};
use gamepad_desktop::runtime::{DesktopRuntime, RuntimeState};
use gamepad_desktop::tray::{PanelControl, TrayAction, TrayMenu, create_tray, gamepad_icon_data};
use gamepad_desktop::{
    control::{ITEM_SHORTCUTS, KEYBOARD_SHORTCUTS, MOUSE_SHORTCUTS, NavigationMode},
    i18n::{Locale, Text},
    keyboard::{self, KeyboardPosition, KeyboardPresentation, KeyboardSession},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

mod keyboard_view;
#[cfg(target_os = "windows")]
mod windows_keyboard_overlay;

type SharedDesktop = Arc<Mutex<DesktopRuntime>>;
const TWO_COLUMN_LAYOUT_MIN_WIDTH: f32 = 960.0;

fn uses_two_columns(available_width: f32) -> bool {
    available_width >= TWO_COLUMN_LAYOUT_MIN_WIDTH
}

struct GamepadDesktopApp {
    desktop: SharedDesktop,
    panel: Arc<PanelControl>,
    tray_error: Option<String>,
    tray_menu: Option<TrayMenu>,
    mapping_text: String,
}

impl GamepadDesktopApp {
    fn new(
        desktop: SharedDesktop,
        panel: Arc<PanelControl>,
        tray_error: Option<String>,
        tray_menu: Option<TrayMenu>,
    ) -> Self {
        Self {
            desktop,
            panel,
            tray_error,
            tray_menu,
            mapping_text: String::new(),
        }
    }
}

impl eframe::App for GamepadDesktopApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        handle_panel_close(ui.ctx(), &self.panel, self.tray_error.is_none());
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        let state = self.desktop.lock().expect("desktop runtime lock").state();
        let locale = state.locale;
        let keyboard = state.keyboard;
        if keyboard.presentation == KeyboardPresentation::Viewport {
            show_keyboard_viewport(ui, keyboard, cfg!(target_os = "windows"));
        }
        egui::Panel::bottom("desktop-footer").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.hyperlink_to(locale.text(Text::Donate), "https://ko-fi.com/andrecrjr");
                ui.separator();
                ui.hyperlink_to(locale.text(Text::Projects), "https://ac-jr.com");
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut state = self.desktop.lock().expect("desktop runtime lock").state();
                let desktop = &self.desktop;
                let panel = &self.panel;
                let tray_error = self.tray_error.as_deref();
                let tray_menu = self.tray_menu.as_ref();
                let mapping_text = &mut self.mapping_text;
                let style = ui.style().clone();
                if uses_two_columns(ui.available_width()) {
                    ui.columns(2, |columns| {
                        egui::Frame::group(&style).show(&mut columns[0], |ui| {
                            show_desktop_overview(ui, &state, desktop);
                        });
                        egui::Frame::group(&style).show(&mut columns[1], |ui| {
                            show_desktop_settings(
                                ui,
                                &mut state,
                                desktop,
                                mapping_text,
                                tray_error,
                                panel,
                                tray_menu,
                            );
                        });
                    });
                } else {
                    egui::Frame::group(&style).show(ui, |ui| {
                        show_desktop_overview(ui, &state, desktop);
                    });
                    ui.add_space(8.0);
                    egui::Frame::group(&style).show(ui, |ui| {
                        show_desktop_settings(
                            ui,
                            &mut state,
                            desktop,
                            mapping_text,
                            tray_error,
                            panel,
                            tray_menu,
                        );
                    });
                }
            });
        });
    }
}

fn show_desktop_overview(ui: &mut egui::Ui, state: &RuntimeState, desktop: &SharedDesktop) {
    let locale = state.locale;
    ui.heading(locale.text(Text::DesktopControl));
    if let Some(error) = &state.error {
        ui.colored_label(Color32::LIGHT_RED, locale.translate(error));
    } else if !state.ready {
        ui.label(locale.text(Text::Connecting));
    } else if let Some(name) = &state.controller {
        ui.colored_label(
            Color32::LIGHT_GREEN,
            format!("{}{name}", locale.text(Text::ControllerPrefix)),
        );
        ui.label(if state.keyboard.open {
            locale.text(Text::KeyboardActive)
        } else if state.settings.enabled {
            match state.mode {
                NavigationMode::Mouse => locale.text(Text::MouseMode),
                NavigationMode::Items => locale.text(Text::ItemsMode),
            }
        } else {
            locale.text(Text::Paused)
        });
    } else {
        ui.label(if state.devices.is_empty() {
            locale.text(Text::ConnectGamepad)
        } else {
            locale.text(Text::SelectController)
        });
        ui.small(locale.text(Text::DeviceSupport));
    }
    if state.error.is_some() && ui.button(locale.text(Text::Retry)).clicked() {
        let runtime = DesktopRuntime::default();
        *desktop.lock().expect("desktop runtime lock") = runtime;
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(if state.keyboard.open {
                locale.text(Text::KeyboardVisual)
            } else if !state.settings.enabled {
                locale.text(Text::PausedLabel)
            } else {
                match state.mode {
                    NavigationMode::Mouse => locale.text(Text::MouseLabel),
                    NavigationMode::Items => locale.text(Text::ItemsLabel),
                }
            })
            .strong()
            .size(20.0)
            .color(Color32::LIGHT_GREEN),
        );
    });
    if state.settings.enabled
        && state.mode == NavigationMode::Items
        && let Some(status) = &state.item_status
    {
        ui.small(locale.translate(status));
    }
    if state.keyboard.presentation != KeyboardPresentation::Unavailable
        && let Some(status) = &state.keyboard.status
    {
        ui.small(locale.translate(status));
    }
    if state.keyboard.open {
        ui.small(locale.text(Text::KeyboardFocusNote));
    }
    if state.keyboard.presentation == KeyboardPresentation::Unavailable
        && let Some(status) = &state.keyboard.status
    {
        ui.colored_label(Color32::LIGHT_RED, locale.translate(status));
    }
    ui.separator();
    ui.heading(locale.text(Text::DesktopCommands));
    shortcut_guide(
        ui,
        locale,
        if state.mode == NavigationMode::Items {
            ITEM_SHORTCUTS
        } else {
            MOUSE_SHORTCUTS
        },
    );
    ui.collapsing(locale.text(Text::KeyboardCommands), |ui| {
        shortcut_guide(ui, locale, KEYBOARD_SHORTCUTS);
    });
    ui.small(locale.text(Text::ButtonPositionNote));
}

fn show_desktop_settings(
    ui: &mut egui::Ui,
    state: &mut RuntimeState,
    desktop: &SharedDesktop,
    mapping_text: &mut String,
    tray_error: Option<&str>,
    panel: &PanelControl,
    tray_menu: Option<&TrayMenu>,
) {
    ui.heading(state.locale.text(Text::Settings));
    let mut locale = state.locale;
    ui.horizontal(|ui| {
        ui.label(state.locale.text(Text::Language));
        egui::ComboBox::from_id_salt("desktop-language")
            .selected_text(locale.name())
            .show_ui(ui, |ui| {
                for option in Locale::ALL {
                    ui.selectable_value(&mut locale, option, option.name());
                }
            });
    });
    if locale != state.locale {
        desktop
            .lock()
            .expect("desktop runtime lock")
            .configure_locale(locale);
        if let Some(tray_menu) = tray_menu {
            tray_menu.set_locale(locale);
        }
        state.locale = locale;
        state.keyboard.locale = locale;
    }
    if let Some(error) = &state.preferences_error {
        ui.small(format!(
            "{}{error}",
            state.locale.text(Text::PreferencesSaveFailedPrefix)
        ));
    }
    let locale = state.locale;
    egui::CollapsingHeader::new(locale.text(Text::ControlsAndAdjustments)).show(ui, |ui| {
        let mut selected = state.selected;
        let selected_name = if selected.is_none() {
            locale.text(Text::Automatic)
        } else {
            state
                .devices
                .iter()
                .find(|device| Some(device.id) == selected)
                .map_or(locale.text(Text::DisconnectedController), |device| {
                    device.name.as_str()
                })
        };
        egui::ComboBox::from_id_salt("desktop-controller")
            .selected_text(selected_name)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, None, locale.text(Text::Automatic));
                for device in &state.devices {
                    let label = format!(
                        "{}{}",
                        device.name,
                        if device.mapped {
                            ""
                        } else {
                            locale.text(Text::NeedsProfile)
                        }
                    );
                    ui.selectable_value(&mut selected, Some(device.id), label);
                }
            });
        if selected != state.selected {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .select_controller(selected);
        }
        let mut changed = ui
            .checkbox(
                &mut state.settings.enabled,
                locale.text(Text::ControlDesktop),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut state.settings.speed, 100.0..=2400.0)
                    .text(locale.text(Text::Speed)),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut state.settings.deadzone, 0.05..=0.5)
                    .text(locale.text(Text::Deadzone)),
            )
            .changed();
        if changed {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .configure(state.settings);
        }
    });
    egui::CollapsingHeader::new(locale.text(Text::CustomizeKeyboard)).show(ui, |ui| {
        let mut settings = state.keyboard.settings;
        let mut changed = ui
            .add(
                egui::Slider::new(&mut settings.width, 520..=1000)
                    .text(locale.text(Text::WidthPx))
                    .step_by(20.0),
            )
            .changed();
        ui.horizontal(|ui| {
            changed |= ui
                .selectable_value(
                    &mut settings.position,
                    KeyboardPosition::Top,
                    locale.text(Text::Top),
                )
                .changed();
            changed |= ui
                .selectable_value(
                    &mut settings.position,
                    KeyboardPosition::Bottom,
                    locale.text(Text::Bottom),
                )
                .changed();
        });
        changed |= ui
            .add(
                egui::Slider::new(&mut settings.opacity, 160..=255)
                    .text(locale.text(Text::Opacity)),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut settings.repeat_enabled,
                locale.text(Text::RepeatDirection),
            )
            .changed();
        ui.add_enabled_ui(settings.repeat_enabled, |ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut settings.repeat_delay_ms, 400..=1200)
                        .text(locale.text(Text::InitialWait))
                        .step_by(50.0),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut settings.repeat_interval_ms, 150..=1000)
                        .text(locale.text(Text::RepeatInterval))
                        .step_by(50.0),
                )
                .changed();
        });
        if ui.button(locale.text(Text::RestoreKeyboard)).clicked() {
            settings = Default::default();
            changed = true;
        }
        if changed {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .configure_keyboard(settings);
        }
        ui.small(locale.text(Text::KeyboardFollowsField));
        ui.small(locale.text(Text::KeyboardSettingsHelp));
        ui.small(locale.text(Text::KeyboardBehaviorHelp));
    });
    if let Some(error) = tray_error {
        ui.colored_label(
            Color32::LIGHT_RED,
            format!(
                "{}{error}. {}",
                locale.text(Text::TrayUnavailablePrefix),
                locale.text(Text::ClosePanelKeepsRunning)
            ),
        );
    } else {
        ui.label(locale.text(Text::ClosePanelKeepsRunning));
    }
    if ui.button(locale.text(Text::QuitApplication)).clicked() {
        panel.exit();
    }
    egui::CollapsingHeader::new(locale.text(Text::ControllerProfiles)).show(ui, |ui| {
        ui.label(locale.text(Text::KnownProfiles));
        for device in &state.devices {
            ui.label(format!("{} · GUID: {}", device.name, device.guid));
        }
        ui.add(
            egui::TextEdit::multiline(mapping_text)
                .hint_text("GUID,nome,a:b0,leftx:a0,lefty:a1,...")
                .desired_rows(2),
        );
        if ui
            .add_enabled(
                !mapping_text.trim().is_empty(),
                egui::Button::new(locale.text(Text::ApplySaveProfile)),
            )
            .clicked()
        {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .import_mapping(mapping_text.clone());
        }
        if let Some(message) = &state.mapping_message {
            ui.label(locale.translate(message));
        }
    });
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

fn show_keyboard_viewport(ui: &mut egui::Ui, session: KeyboardSession, immediate: bool) {
    let locale = session.locale;
    let screen = ui
        .ctx()
        .input(|input| input.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    let (width, height) = session.settings.surface_size();
    let viewport = egui::ViewportId::from_hash_of("gamepad-desktop-keyboard");
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
    let builder = keyboard_viewport_builder(
        size,
        position,
        session.open,
        cfg!(target_os = "windows"),
        locale,
    );
    if immediate {
        ui.ctx()
            .show_viewport_immediate(viewport, builder, move |ui, _| {
                keyboard_viewport_ui(ui, &session)
            });
    } else {
        ui.ctx()
            .show_viewport_deferred(viewport, builder, move |ui, _| {
                keyboard_viewport_ui(ui, &session)
            });
    }
}

fn keyboard_viewport_builder(
    size: egui::Vec2,
    position: egui::Pos2,
    open: bool,
    windows: bool,
    locale: Locale,
) -> egui::ViewportBuilder {
    let builder = egui::ViewportBuilder::default()
        .with_title(locale.text(Text::KeyboardWindowTitle))
        .with_inner_size(size)
        .with_position(position)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_active(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_has_shadow(false)
        .with_visible(open);
    if windows {
        // winit sets WS_EX_LAYERED for mouse passthrough on Windows, hiding
        // transparent overlay content. The Windows helper applies click-through
        // without that style after the viewport exists.
        builder
    } else {
        builder.with_mouse_passthrough(true)
    }
}

fn main_viewport_builder(windows: bool) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title("Gamepad Desktop")
        .with_icon(gamepad_icon_data(512))
        .with_inner_size([860.0, 760.0])
        .with_min_inner_size([660.0, 500.0])
        // eframe shares the root GL config with child viewports.
        .with_transparent(windows)
}

#[cfg(test)]
mod overlay_tests {
    use super::*;

    #[test]
    fn windows_keyboard_viewport_renders_during_the_parent_pass() {
        let context = egui::Context::default();
        context.set_embed_viewports(false);
        let rendered_immediately = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let rendered = Arc::clone(&rendered_immediately);
        egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
            rendered.store(true, std::sync::atomic::Ordering::Relaxed);
            let mut input = egui::RawInput {
                viewport_id: viewport.ids.this,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 258.0),
                )),
                ..Default::default()
            };
            input.viewports.insert(
                viewport.ids.this,
                egui::ViewportInfo {
                    parent: Some(viewport.ids.parent),
                    ..Default::default()
                },
            );
            let mut output = context.run_ui(input, |ui| (viewport.viewport_ui_cb)(ui));
            output.textures_delta.clear();
        });
        let mut session = KeyboardSession::default();
        session.open = true;

        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 258.0),
                )),
                ..Default::default()
            },
            |ui| show_keyboard_viewport(ui, session.clone(), true),
        );
        output.textures_delta.clear();

        assert!(rendered_immediately.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn windows_root_viewport_requests_transparency_for_keyboard_overlay() {
        assert_eq!(main_viewport_builder(true).transparent, Some(true));
        assert_eq!(main_viewport_builder(false).transparent, Some(false));
    }

    #[test]
    fn windows_keyboard_viewport_avoids_winit_mouse_passthrough() {
        let builder = keyboard_viewport_builder(
            egui::vec2(640.0, 229.0),
            egui::Pos2::ZERO,
            true,
            true,
            Locale::default(),
        );
        assert_eq!(builder.mouse_passthrough, None);
    }

    #[test]
    fn closed_keyboard_has_no_visible_window_or_native_shadow() {
        let builder = keyboard_viewport_builder(
            egui::vec2(640.0, 229.0),
            egui::Pos2::ZERO,
            false,
            false,
            Locale::default(),
        );
        assert_eq!(builder.visible, Some(false));
        assert_eq!(builder.has_shadow, Some(false));
        assert_eq!(builder.transparent, Some(true));
        assert_eq!(builder.active, Some(false));
    }
}

fn keyboard_viewport_ui(ui: &mut egui::Ui, session: &KeyboardSession) {
    #[cfg(target_os = "windows")]
    if session.open {
        windows_keyboard_overlay::enable_click_through();
    }
    ui.ctx().request_repaint_after(Duration::from_millis(50));
    keyboard_view::show(ui, session);
}

fn shortcut_guide(ui: &mut egui::Ui, locale: Locale, shortcuts: &[(&str, &str)]) {
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .striped(true)
        .spacing([24.0, 9.0])
        .show(ui, |ui| {
            for (button, action) in shortcuts {
                let button = locale.translate(button);
                let action = locale.translate(action);
                ui.label(
                    RichText::new(button)
                        .strong()
                        .color(Color32::from_rgb(100, 220, 200)),
                );
                ui.label(action);
                ui.end_row();
            }
        });
}

#[cfg(target_os = "linux")]
fn acquire_instance_lock() -> std::io::Result<Option<std::fs::File>> {
    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cache"))
        })
        .ok_or_else(|| std::io::Error::other("XDG_RUNTIME_DIR and HOME are unavailable"))?
        .join("gamepad-desktop");
    std::fs::create_dir_all(&directory)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("desktop.lock"))?;
    match lock.try_lock() {
        Ok(()) => Ok(Some(lock)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

fn main() -> eframe::Result {
    #[cfg(target_os = "linux")]
    let _instance_lock = match acquire_instance_lock() {
        Ok(Some(lock)) => lock,
        Ok(None) => return Ok(()),
        Err(error) => return Err(eframe::Error::AppCreation(Box::new(error))),
    };

    // The service belongs to the process, so closing a Wayland window keeps it alive.
    let desktop = Arc::new(Mutex::new(DesktopRuntime::default()));
    let (panel, actions) = PanelControl::new();
    let locale = desktop.lock().expect("desktop runtime lock").state().locale;
    let tray_result = create_tray(Arc::clone(&panel), locale);
    let tray_error = tray_result.as_ref().err().cloned();
    let tray_menu = tray_result.as_ref().ok().map(|(_, menu)| menu.clone());
    let tray = tray_result.map(|(icon, _)| icon);
    loop {
        let options = eframe::NativeOptions {
            viewport: main_viewport_builder(cfg!(target_os = "windows")),
            run_and_return: true,
            ..Default::default()
        };

        let app_desktop = Arc::clone(&desktop);
        let app_panel = Arc::clone(&panel);
        let app_error = tray_error.clone();
        let app_tray_menu = tray_menu.clone();
        eframe::run_native(
            "Gamepad Desktop",
            options,
            Box::new(move |creation_context| {
                creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
                app_panel.attach(Some(creation_context.egui_ctx.clone()));
                Ok(Box::new(GamepadDesktopApp::new(
                    app_desktop,
                    app_panel,
                    app_error,
                    app_tray_menu,
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
                shortcut_guide(ui, Locale::default(), MOUSE_SHORTCUTS);
                assert!(
                    ui.min_rect().right() <= 660.0,
                    "guide overflows: {:?}",
                    ui.min_rect()
                );
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn default_window_width_uses_a_single_column_layout() {
        assert!(!uses_two_columns(860.0));
        assert!(uses_two_columns(TWO_COLUMN_LAYOUT_MIN_WIDTH));
    }
}
