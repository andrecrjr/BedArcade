use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{
    control::{Action, ControlSettings, GamepadFrame, MouseController},
    gamepads::{GamepadInfo, Gamepads},
    i18n::Locale,
    keyboard::{KeyboardSession, KeyboardSettings},
    platform::Platform,
    preferences::DesktopPreferences,
};

const ACTIVE_POLL_INTERVAL: Duration = Duration::from_millis(8);
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(16);

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
                update_gamepad_state(&mut state, frame.is_some(), &gamepads.devices, || {
                    gamepads.name()
                });
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
                    action => platform.apply(action)?,
                }
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
            thread::sleep(poll_interval(frame.is_some()));
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

fn update_gamepad_state(
    state: &mut RuntimeState,
    connected: bool,
    devices: &[GamepadInfo],
    name: impl FnOnce() -> Option<String>,
) {
    if connected {
        if state.controller.is_none() {
            state.controller = name();
        }
    } else {
        state.controller = None;
    }
    if !same_devices(&state.devices, devices) {
        state.devices.clear();
        state.devices.extend_from_slice(devices);
    }
    state.ready = true;
}

fn same_devices(left: &[GamepadInfo], right: &[GamepadInfo]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.id == right.id
                && left.name == right.name
                && left.guid == right.guid
                && left.mapped == right.mapped
        })
}

fn poll_interval(connected: bool) -> Duration {
    if connected {
        ACTIVE_POLL_INTERVAL
    } else {
        IDLE_POLL_INTERVAL
    }
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
    publish_keyboard_state(&mut state.keyboard, &controller.keyboard);
    actions
}

fn publish_keyboard_state(state: &mut KeyboardSession, keyboard: &KeyboardSession) {
    if *state != *keyboard {
        state.clone_from(keyboard);
    }
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

    #[test]
    fn runtime_keeps_the_active_poll_rate_and_backs_off_when_disconnected() {
        assert_eq!(poll_interval(true), Duration::from_millis(8));
        assert_eq!(poll_interval(false), Duration::from_millis(16));
    }

    #[test]
    fn unchanged_gamepad_snapshot_does_not_recreate_the_name_or_device_list() {
        let devices = vec![GamepadInfo {
            id: 7,
            name: "Virtual controller".to_owned(),
            guid: "0123456789abcdef0123456789abcdef".to_owned(),
            mapped: true,
        }];
        let mut state = RuntimeState::default();
        let mut name_reads = 0;

        update_gamepad_state(&mut state, true, &devices, || {
            name_reads += 1;
            Some("Virtual controller".to_owned())
        });
        update_gamepad_state(&mut state, true, &devices, || {
            name_reads += 1;
            Some("Virtual controller".to_owned())
        });

        assert_eq!(name_reads, 1);
        assert_eq!(state.controller.as_deref(), Some("Virtual controller"));
        assert!(same_devices(&state.devices, &devices));
        assert!(state.ready);
    }

    #[test]
    #[ignore = "manual benchmark comparing unchanged runtime-state copies"]
    fn runtime_state_copy_benchmark() {
        use std::hint::black_box;

        const ITERATIONS: usize = 200_000;
        let mut keyboard = KeyboardSession::default();
        keyboard.status = Some("Keyboard ready".to_owned());
        for device_count in [1, 4] {
            let devices: Vec<_> = (0..device_count)
                .map(|id| GamepadInfo {
                    id,
                    name: format!("Virtual controller {id}"),
                    guid: format!("{id:032x}"),
                    mapped: true,
                })
                .collect();
            let mut initial = RuntimeState {
                controller: Some("Virtual controller 0".to_owned()),
                devices: devices.clone(),
                keyboard: keyboard.clone(),
                ..RuntimeState::default()
            };
            let name = "Virtual controller 0";

            let baseline_started = Instant::now();
            for _ in 0..ITERATIONS {
                initial.controller = Some(name.to_owned());
                initial.devices = devices.clone();
                initial.keyboard = keyboard.clone();
                black_box(&initial);
            }
            let baseline_ns = baseline_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let mut optimized = RuntimeState {
                controller: Some(name.to_owned()),
                devices: devices.clone(),
                keyboard: keyboard.clone(),
                ..RuntimeState::default()
            };
            let optimized_started = Instant::now();
            for _ in 0..ITERATIONS {
                update_gamepad_state(&mut optimized, true, &devices, || Some(name.to_owned()));
                publish_keyboard_state(&mut optimized.keyboard, &keyboard);
                black_box(&optimized);
            }
            let optimized_ns = optimized_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let baseline_ui_started = Instant::now();
            for _ in 0..ITERATIONS {
                let first = initial.clone();
                black_box((first.locale, first.keyboard));
                black_box(initial.clone());
            }
            let baseline_ui_ns =
                baseline_ui_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let optimized_ui_started = Instant::now();
            for _ in 0..ITERATIONS {
                let snapshot = optimized.clone();
                black_box((snapshot.locale, snapshot.keyboard.clone()));
                black_box(snapshot);
            }
            let optimized_ui_ns =
                optimized_ui_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            eprintln!(
                "devices={device_count} tick_ns baseline={baseline_ns:.1} optimized={optimized_ns:.1} ui_ns baseline={baseline_ui_ns:.1} optimized={optimized_ui_ns:.1}"
            );
        }
    }
}
