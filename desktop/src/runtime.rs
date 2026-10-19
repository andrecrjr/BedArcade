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
    keyboard::{KeyboardSession, KeyboardSettings},
    platform::Platform,
};

#[derive(Clone, Default)]
pub struct RuntimeState {
    pub settings: ControlSettings,
    pub controller: Option<String>,
    pub error: Option<String>,
    pub ready: bool,
    pub devices: Vec<GamepadInfo>,
    pub selected: Option<u32>,
    pub mapping_message: Option<String>,
    pub keyboard: KeyboardSession,
    pending_mapping: Option<String>,
}

pub struct DesktopRuntime {
    state: Arc<Mutex<RuntimeState>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Default for DesktopRuntime {
    fn default() -> Self {
        let mut initial = RuntimeState::default();
        initial.keyboard.settings = KeyboardSettings::load();
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
    }
}

fn run(state: &Mutex<RuntimeState>, stop: &AtomicBool) -> Result<(), String> {
    let input_tracker = crate::focused_input::InputTracker::default();
    let mut platform = Platform::new()?;
    let mut gamepads = Gamepads::new().map_err(|error| format!("SDL3: {error}"))?;
    state.lock().expect("runtime state lock").mapping_message = gamepads.warning.take();
    let mut controller = MouseController::default();
    controller.keyboard.settings = state.lock().expect("runtime state lock").keyboard.settings;
    let mut saved_settings = controller.keyboard.settings;
    controller.set_system_keyboard_uses_gamepad(platform.system_keyboard_uses_gamepad());
    controller.keyboard.presentation = platform.keyboard_presentation();
    controller.keyboard.status = platform.keyboard_overlay_error().map(str::to_owned);
    state.lock().expect("runtime state lock").keyboard = controller.keyboard.clone();
    let mut last_tick = Instant::now();
    let outcome = (|| {
        while !stop.load(Ordering::Relaxed) {
            let now = Instant::now();
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
                if let Action::OnScreenKeyboard(open) = action {
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
                            "Teclado visual próprio · direcional move · A digita · B apaga · L3 fecha."
                        } else {
                            "Teclado visual fechado."
                        }.to_owned());
                    }
                    let mut runtime_state = state.lock().expect("runtime state lock");
                    runtime_state.keyboard = controller.keyboard.clone();
                } else {
                    platform.apply(action)?;
                }
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
    let actions = controller.update(frame, elapsed, &mut state.settings);
    state.keyboard = controller.keyboard.clone();
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::GamepadFrame;

    #[test]
    fn l3_requests_the_system_keyboard_without_opening_the_app_panel() {
        let mut state = RuntimeState::default();
        let mut controller = MouseController::default();
        let neutral = GamepadFrame::default();
        update_control(&mut state, &mut controller, Some(&neutral), Duration::ZERO);
        let mut frame = neutral;
        frame.buttons[crate::control::Button::LeftStick as usize] = true;

        let actions = update_control(&mut state, &mut controller, Some(&frame), Duration::ZERO);

        assert_eq!(actions, vec![Action::OnScreenKeyboard(true)]);
        assert!(state.keyboard.open);
        assert!(controller.keyboard.open);
    }
}
