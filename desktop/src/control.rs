use std::time::Duration;

use crate::keyboard::{KeyboardKey, KeyboardSession};

const REPEAT_INITIAL_DELAY: f64 = 0.4;
const DIRECTION_REPEAT_INTERVAL: f64 = 0.08;
const KEYBOARD_REPEAT_INTERVAL: f64 = 0.18;

pub const MOUSE_SHORTCUTS: &[(&str, &str)] = &[
    (
        "L3 · pressionar analógico esquerdo",
        "Abrir / fechar teclado visual próprio",
    ),
    (
        "R3 · pressionar analógico direito",
        "Lançador do sistema · Super / Windows",
    ),
    ("Analógico esquerdo", "Mover ponteiro"),
    ("Analógico direito ↕", "Rolar página"),
    (
        "Inferior · A / ✕ / B no Switch",
        "Clique esquerdo · segurar para arrastar",
    ),
    ("Direito · B / ○ / A no Switch", "Clique direito"),
    (
        "Esquerdo · X / □ / Y no Switch",
        "Enter · abrir item selecionado",
    ),
    (
        "Superior · Y / △ / X no Switch",
        "Escape · fechar menu / cancelar",
    ),
    ("Direcional", "Setas · segurar para repetir"),
    ("LB / L1", "Tab · próximo campo"),
    ("RB / R1", "Alt+Tab · alternar janela"),
    (
        "Start / Menu / +",
        "Pausar / retomar · solte e centralize para retomar",
    ),
];

pub const KEYBOARD_SHORTCUTS: &[(&str, &str)] = &[
    ("Direcional", "Mover a seleção pelo teclado"),
    ("A / botão inferior", "Digitar a tecla selecionada"),
    ("B / botão direito", "Apagar um caractere"),
    ("X / botão esquerdo", "Inserir Enter"),
    ("Y / botão superior", "Inserir Esc"),
    ("R3", "Inserir espaço"),
    ("L1", "Inserir Tab"),
    ("R1", "Ativar ou desativar Shift"),
    ("L3", "Fechar o teclado visual"),
];

#[derive(Clone, Copy, Debug)]
pub enum Button {
    South,
    East,
    West,
    North,
    Up,
    Down,
    Left,
    Right,
    LeftShoulder,
    RightShoulder,
    RightStick,
    Start,
    LeftStick,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadFrame {
    pub x: f64,
    pub y: f64,
    pub scroll: f64,
    pub buttons: [bool; 13],
}

impl GamepadFrame {
    pub fn pressed(&self, button: Button) -> bool {
        self.buttons[button as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlSettings {
    pub enabled: bool,
    pub speed: f64,
    pub deadzone: f64,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            speed: 900.0,
            deadzone: 0.2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemKey {
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Tab,
    Meta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardInput {
    Character(char),
    Backspace,
    Enter,
    Escape,
    Space,
    Tab,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(i32, i32),
    Scroll(i32),
    Mouse(MouseButton, bool),
    Key(SystemKey),
    Keyboard(KeyboardInput),
    NextWindow,
    OnScreenKeyboard(bool),
}

#[derive(Default)]
pub struct MouseController {
    pub keyboard: KeyboardSession,
    previous: GamepadFrame,
    connected: bool,
    armed: bool,
    held: [bool; 2],
    remainder: [f64; 3],
    repeat: [f64; 4],
    system_keyboard_uses_gamepad: bool,
}

impl MouseController {
    pub fn set_system_keyboard_uses_gamepad(&mut self, enabled: bool) {
        self.system_keyboard_uses_gamepad = enabled;
    }

    pub fn update(
        &mut self,
        frame: Option<&GamepadFrame>,
        elapsed: Duration,
        settings: &mut ControlSettings,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let Some(frame) = frame else {
            self.close_keyboard(&mut actions);
            self.release(&mut actions);
            self.connected = false;
            self.armed = false;
            self.previous = GamepadFrame::default();
            return actions;
        };
        if !self.connected {
            self.previous = *frame;
            self.connected = true;
        }
        if !(self.keyboard.open && self.system_keyboard_uses_gamepad)
            && frame.pressed(Button::Start)
            && !self.previous.pressed(Button::Start)
        {
            settings.enabled = !settings.enabled;
            self.armed = false;
        }
        if !settings.enabled || !self.armed {
            if !settings.enabled {
                self.close_keyboard(&mut actions);
            }
            self.release(&mut actions);
            self.armed = settings.enabled
                && !frame.buttons.iter().any(|pressed| *pressed)
                && [frame.x, frame.y, frame.scroll]
                    .iter()
                    .all(|axis| axis.is_finite() && axis.abs() <= settings.deadzone);
            self.previous = *frame;
            return actions;
        }

        let keyboard_toggled =
            frame.pressed(Button::LeftStick) && !self.previous.pressed(Button::LeftStick);
        if keyboard_toggled {
            actions.push(self.toggle_keyboard());
            self.release(&mut actions);
            self.previous = *frame;
        }
        if self.keyboard.open && self.system_keyboard_uses_gamepad && !keyboard_toggled {
            self.release(&mut actions);
            self.update_keyboard(frame, elapsed, &mut actions);
            self.previous = *frame;
            return actions;
        }

        for (index, (button, mouse)) in [
            (Button::South, MouseButton::Left),
            (Button::East, MouseButton::Right),
        ]
        .into_iter()
        .enumerate()
        {
            let pressed = frame.pressed(button);
            if !keyboard_toggled && pressed != self.held[index] {
                actions.push(Action::Mouse(mouse, pressed));
                self.held[index] = pressed;
            }
        }
        for (button, action) in [
            (Button::West, Action::Key(SystemKey::Enter)),
            (Button::North, Action::Key(SystemKey::Escape)),
            (Button::RightStick, Action::Key(SystemKey::Meta)),
            (Button::LeftShoulder, Action::Key(SystemKey::Tab)),
            (Button::RightShoulder, Action::NextWindow),
        ] {
            if !keyboard_toggled && frame.pressed(button) && !self.previous.pressed(button) {
                actions.push(action);
            }
        }
        let seconds = elapsed.as_secs_f64().min(0.05);
        for (index, (button, key)) in [
            (Button::Up, SystemKey::Up),
            (Button::Down, SystemKey::Down),
            (Button::Left, SystemKey::Left),
            (Button::Right, SystemKey::Right),
        ]
        .into_iter()
        .enumerate()
        {
            if !keyboard_toggled
                && self.repeat_direction(frame, button, index, elapsed, DIRECTION_REPEAT_INTERVAL)
            {
                actions.push(Action::Key(key));
            }
        }
        let x = axis_value(frame.x, settings.deadzone);
        let y = axis_value(frame.y, settings.deadzone);
        let magnitude = x.hypot(y).max(1.0);
        let speed = settings.speed.clamp(100.0, 2400.0);
        let dx = accumulate(&mut self.remainder[0], x / magnitude * speed * seconds);
        let dy = accumulate(&mut self.remainder[1], y / magnitude * speed * seconds);
        if dx != 0 || dy != 0 {
            actions.push(Action::Move(dx, dy));
        }
        let scroll = accumulate(
            &mut self.remainder[2],
            axis_value(frame.scroll, settings.deadzone) * 10.0 * seconds,
        );
        if scroll != 0 {
            actions.push(Action::Scroll(scroll));
        }
        self.previous = *frame;
        actions
    }

    fn release(&mut self, actions: &mut Vec<Action>) {
        self.release_mouse(actions);
        self.remainder = [0.0; 3];
        self.repeat = [0.0; 4];
    }

    fn toggle_keyboard(&mut self) -> Action {
        self.keyboard.open = !self.keyboard.open;
        self.keyboard.status = None;
        Action::OnScreenKeyboard(self.keyboard.open)
    }

    fn close_keyboard(&mut self, actions: &mut Vec<Action>) {
        if self.keyboard.open {
            self.keyboard.open = false;
            actions.push(Action::OnScreenKeyboard(false));
        }
    }

    fn release_mouse(&mut self, actions: &mut Vec<Action>) {
        for (index, button) in [MouseButton::Left, MouseButton::Right]
            .into_iter()
            .enumerate()
        {
            if self.held[index] {
                actions.push(Action::Mouse(button, false));
            }
        }
        self.held = [false; 2];
    }

    fn repeat_direction(
        &mut self,
        frame: &GamepadFrame,
        button: Button,
        index: usize,
        elapsed: Duration,
        repeat_interval: f64,
    ) -> bool {
        if !frame.pressed(button) {
            return false;
        }
        if !self.previous.pressed(button) {
            self.repeat[index] = REPEAT_INITIAL_DELAY;
            return true;
        }
        self.repeat[index] -= elapsed.as_secs_f64().min(0.05);
        if self.repeat[index] > 0.0 {
            return false;
        }
        self.repeat[index] += repeat_interval;
        true
    }

    fn update_keyboard(
        &mut self,
        frame: &GamepadFrame,
        elapsed: Duration,
        actions: &mut Vec<Action>,
    ) {
        for (index, (button, row_delta, column_delta)) in [
            (Button::Up, -1, 0),
            (Button::Down, 1, 0),
            (Button::Left, 0, -1),
            (Button::Right, 0, 1),
        ]
        .into_iter()
        .enumerate()
        {
            if self.repeat_direction(frame, button, index, elapsed, KEYBOARD_REPEAT_INTERVAL) {
                self.keyboard.move_selection(row_delta, column_delta);
            }
        }

        for (button, input) in [
            (Button::East, KeyboardInput::Backspace),
            (Button::West, KeyboardInput::Enter),
            (Button::North, KeyboardInput::Escape),
            (Button::RightStick, KeyboardInput::Space),
            (Button::LeftShoulder, KeyboardInput::Tab),
        ] {
            if frame.pressed(button) && !self.previous.pressed(button) {
                actions.push(Action::Keyboard(input));
            }
        }

        if frame.pressed(Button::RightShoulder) && !self.previous.pressed(Button::RightShoulder) {
            self.keyboard.shift = !self.keyboard.shift;
        }
        if frame.pressed(Button::South) && !self.previous.pressed(Button::South) {
            match self.keyboard.selected_key() {
                key @ KeyboardKey::Character(_) => {
                    let character = key.character(self.keyboard.shift).unwrap();
                    actions.push(Action::Keyboard(KeyboardInput::Character(character)));
                    self.keyboard.shift = false;
                }
                KeyboardKey::Backspace => actions.push(Action::Keyboard(KeyboardInput::Backspace)),
                KeyboardKey::Enter => actions.push(Action::Keyboard(KeyboardInput::Enter)),
                KeyboardKey::Escape => actions.push(Action::Keyboard(KeyboardInput::Escape)),
                KeyboardKey::Shift => self.keyboard.shift = !self.keyboard.shift,
                KeyboardKey::Space => actions.push(Action::Keyboard(KeyboardInput::Space)),
                KeyboardKey::Tab => actions.push(Action::Keyboard(KeyboardInput::Tab)),
            }
        }
    }
}

fn axis_value(value: f64, deadzone: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let deadzone = deadzone.clamp(0.05, 0.5);
    let value = value.clamp(-1.0, 1.0);
    value.signum() * ((value.abs() - deadzone).max(0.0) / (1.0 - deadzone)).powi(2)
}

fn accumulate(remainder: &mut f64, amount: f64) -> i32 {
    *remainder += amount;
    let whole = remainder.trunc() as i32;
    *remainder -= f64::from(whole);
    whole
}
