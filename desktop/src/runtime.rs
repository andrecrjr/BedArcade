use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{
    control::{Action, ControlSettings, GamepadFrame, MouseController, NavigationMode, SystemKey},
    gamepads::{GamepadInfo, Gamepads},
    i18n::Locale,
    item_navigation::ItemNavigator,
    keyboard::{KeyboardSession, KeyboardSettings},
    platform::Platform,
    preferences::DesktopPreferences,
};

#[derive(Clone, Default)]
pub struct RuntimeState {
    pub locale: Locale,
    pub settings: ControlSettings,
    pub controller: Option<String>,
    pub error: Option<String>,
    pub ready: bool,
    pub devices: Vec<GamepadInfo>,
    pub selected: Option<u32>,
    pub mapping_message: Option<String>,
    pub keyboard: KeyboardSession,
    pub mode: NavigationMode,
    pub item_status: Option<String>,
    pub preferences_error: Option<String>,
    pending_mapping: Option<String>,
}

pub struct DesktopRuntime {
    state: Arc<Mutex<RuntimeState>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Default for DesktopRuntime {
    fn default() -> Self {
        let preferences = DesktopPreferences::load();
        let initial = RuntimeState {
            locale: preferences.locale,
            settings: preferences.controls,
            keyboard: KeyboardSession::with_locale(preferences.locale, KeyboardSettings::load()),
            ..RuntimeState::default()
        };
        let state = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&state);
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            if let Err(error) = run(&worker_state, &worker_stop) {
                let mut state = worker_state.lock().expect("runtime state lock");
                state.error = Some(error);
                state.ready = false;
                state.keyboard.open = false;
            }
        });
        Self {
            state,
            stop,
            worker: Some(worker),
        }
    }
}

impl DesktopRuntime {
    pub fn state(&self) -> RuntimeState {
        self.state.lock().expect("runtime state lock").clone()
    }

    pub fn configure(&self, settings: ControlSettings) {
        self.state.lock().expect("runtime state lock").settings = settings;
    }

    pub fn configure_keyboard(&self, settings: KeyboardSettings) {
        self.state
            .lock()
            .expect("runtime state lock")
            .keyboard
            .settings = settings.normalized();
    }

    pub fn configure_locale(&self, locale: Locale) {
        let mut state = self.state.lock().expect("runtime state lock");
        state.locale = locale;
        state.keyboard.locale = locale;
    }

    pub fn save_preferences(&self) -> Result<(), String> {
        let state = self.state.lock().expect("runtime state lock");
        DesktopPreferences {
            locale: state.locale,
            controls: state.settings,
        }
        .save()
    }

    pub fn select_controller(&self, selected: Option<u32>) {
        self.state.lock().expect("runtime state lock").selected = selected;
    }

    pub fn import_mapping(&self, mapping: String) {
        self.state
            .lock()
            .expect("runtime state lock")
            .pending_mapping = Some(mapping);
    }
}

impl Drop for DesktopRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = self.save_preferences();
    }
}

fn run(state: &Mutex<RuntimeState>, stop: &AtomicBool) -> Result<(), String> {
    let input_tracker = crate::focused_input::InputTracker::default();
    let mut platform = Platform::new()?;
    let mut gamepads = Gamepads::new().map_err(|error| format!("SDL3: {error}"))?;
    let item_navigation = ItemNavigator::default();
    state.lock().expect("runtime state lock").mapping_message = gamepads.warning.take();
    let mut controller = MouseController::default();
    controller.keyboard.settings = state.lock().expect("runtime state lock").keyboard.settings;
    controller.keyboard.locale = state.lock().expect("runtime state lock").locale;
    let mut saved_settings = controller.keyboard.settings;
    controller.set_system_keyboard_uses_gamepad(platform.system_keyboard_uses_gamepad());
    controller.keyboard.presentation = platform.keyboard_presentation();
    controller.keyboard.status = platform.keyboard_overlay_error().map(str::to_owned);
    state.lock().expect("runtime state lock").keyboard = controller.keyboard.clone();
    let mut saved_preferences = {
        let state = state.lock().expect("runtime state lock");
        DesktopPreferences {
            locale: state.locale,
            controls: state.settings,
        }
    };
    let mut pending_preferences: Option<(DesktopPreferences, Instant)> = None;
    let mut preference_save: Option<(DesktopPreferences, JoinHandle<Result<(), String>>)> = None;
    let mut last_tick = Instant::now();
    let outcome = (|| {
        while !stop.load(Ordering::Relaxed) {
            let now = Instant::now();
            if let Some((preferences, worker)) = preference_save.take() {
                if worker.is_finished() {
                    match worker.join() {
                        Ok(Ok(())) => {
                            saved_preferences = preferences;
                            state.lock().expect("runtime state lock").preferences_error = None;
                        }
                        Ok(Err(error)) => {
                            state.lock().expect("runtime state lock").preferences_error =
                                Some(error);
                            if pending_preferences.is_none() {
                                pending_preferences = Some((preferences, now));
                            }
                        }
                        Err(_) => {
                            state.lock().expect("runtime state lock").preferences_error =
                                Some("Preference save worker stopped unexpectedly".to_owned());
                            if pending_preferences.is_none() {
                                pending_preferences = Some((preferences, now));
                            }
                        }
                    }
                } else {
                    preference_save = Some((preferences, worker));
                }
            }
            let (selected, mapping) = {
                let mut state = state.lock().expect("runtime state lock");
                (state.selected, state.pending_mapping.take())
            };
            let importing = mapping.is_some();
            if let Some(mapping) = mapping {
                let message = match gamepads.import_mapping(&mapping) {
                    Ok(()) => "Perfil aplicado e salvo para as próximas sessões.".to_owned(),
                    Err(error) => error,
                };
                state.lock().expect("runtime state lock").mapping_message = Some(message);
            }
            let frame = if importing {
                None
            } else {
                gamepads.poll(selected)?
            };
            let actions = {
                controller.keyboard.focused_input = input_tracker.latest();
                let mut state = state.lock().expect("runtime state lock");
                state.controller = gamepads.name();
                state.devices = gamepads.devices.clone();
                state.ready = true;
                update_control(
                    &mut state,
                    &mut controller,
                    frame.as_ref(),
                    now.duration_since(last_tick),
                )
            };
            last_tick = now;
            for action in actions {
                match action {
                    Action::OnScreenKeyboard(open) => {
                        if open
                            && controller.keyboard.presentation
                                == crate::keyboard::KeyboardPresentation::Unavailable
                        {
                            controller.keyboard.open = false;
                            controller.keyboard.status = platform
                                .keyboard_overlay_error()
                                .map(str::to_owned)
                                .or_else(|| Some("Este compositor não permite exibir o teclado próprio sem ativá-lo.".to_owned()));
                        } else {
                            controller.keyboard.status = Some(if open {
                                "Teclado visual próprio · direcional move · A digita · B apaga · R3 fecha."
                            } else {
                                "Teclado visual fechado."
                            }.to_owned());
                        }
                        let mut runtime_state = state.lock().expect("runtime state lock");
                        runtime_state.keyboard = controller.keyboard.clone();
                    }
                    Action::ModeChanged(mode) => {
                        item_navigation.set_enabled(mode == NavigationMode::Items)
                    }
                    Action::ItemDirection(direction) => item_navigation.move_focus(direction),
                    Action::ItemActivate => item_navigation.activate(),
                    Action::ItemCancel => {
                        platform.apply(Action::Key(SystemKey::Escape))?;
                        item_navigation.refresh();
                    }
                    Action::Key(key) => {
                        platform.apply(Action::Key(key))?;
                        if controller.mode == NavigationMode::Items {
                            item_navigation.refresh();
                        }
                    }
                    Action::NextWindow => {
                        platform.apply(Action::NextWindow)?;
                        if controller.mode == NavigationMode::Items {
                            item_navigation.refresh();
                        }
                    }
                    action => {
                        platform.apply(action)?;
                    }
                }
            }
            {
                let mut runtime_state = state.lock().expect("runtime state lock");
                runtime_state.mode = controller.mode;
                runtime_state.item_status =
                    (controller.mode == NavigationMode::Items).then(|| item_navigation.status());
            }
            let preferences = {
                let state = state.lock().expect("runtime state lock");
                DesktopPreferences {
                    locale: state.locale,
                    controls: state.settings,
                }
            };
            if preferences == saved_preferences {
                pending_preferences = None;
            } else if pending_preferences
                .map(|(pending, _)| pending != preferences)
                .unwrap_or(true)
            {
                pending_preferences = Some((preferences, now));
            }
            if preference_save.is_none()
                && let Some((pending, changed_at)) = pending_preferences
                && now.duration_since(changed_at) >= Duration::from_millis(750)
            {
                preference_save = Some((pending, thread::spawn(move || pending.save())));
                pending_preferences = None;
            }
            if let Err(error) = platform.update_keyboard_overlay(&controller.keyboard) {
                controller.keyboard.open = false;
                controller.keyboard.presentation =
                    crate::keyboard::KeyboardPresentation::Unavailable;
                controller.keyboard.status = Some(error);
                state.lock().expect("runtime state lock").keyboard = controller.keyboard.clone();
            }
            if !controller.keyboard.customizing && controller.keyboard.settings != saved_settings {
                saved_settings = controller.keyboard.settings;
                if let Err(error) = saved_settings.save() {
                    controller.keyboard.status =
                        Some(format!("Não foi possível salvar ajustes: {error}"));
                }
            }
            thread::sleep(Duration::from_millis(8));
        }
        Ok(())
    })();
    for action in controller.update(None, Duration::ZERO, &mut ControlSettings::default()) {
        if !matches!(action, Action::OnScreenKeyboard(_)) {
            let _ = platform.apply(action);
        }
    }
    if let Some((_, worker)) = preference_save {
        let _ = worker.join();
    }
    let _ = platform.update_keyboard_overlay(&controller.keyboard);
    if controller.keyboard.settings != saved_settings {
        let _ = controller.keyboard.settings.save();
    }
    outcome
}

fn update_control(
    state: &mut RuntimeState,
    controller: &mut MouseController,
    frame: Option<&GamepadFrame>,
    elapsed: Duration,
) -> Vec<Action> {
    controller.keyboard.settings = state.keyboard.settings;
    controller.keyboard.locale = state.locale;
    let actions = controller.update(frame, elapsed, &mut state.settings);
    state.mode = controller.mode;
    state.keyboard = controller.keyboard.clone();
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::GamepadFrame;

    #[test]
    fn r3_requests_the_system_keyboard_without_opening_the_app_panel() {
        let mut state = RuntimeState::default();
        let mut controller = MouseController::default();
        let neutral = GamepadFrame::default();
        update_control(&mut state, &mut controller, Some(&neutral), Duration::ZERO);
        let mut frame = neutral;
        frame.buttons[crate::control::Button::RightStick as usize] = true;

        let actions = update_control(&mut state, &mut controller, Some(&frame), Duration::ZERO);

        assert_eq!(actions, vec![Action::OnScreenKeyboard(true)]);
        assert!(state.keyboard.open);
        assert!(controller.keyboard.open);
    }
}
