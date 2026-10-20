use std::time::Duration;

use crate::keyboard::{KeyboardKey, KeyboardModifiers, KeyboardSession};

const REPEAT_INITIAL_DELAY: f64 = 0.4;
const DIRECTION_REPEAT_INTERVAL: f64 = 0.08;

pub const MOUSE_SHORTCUTS: &[(&str, &str)] = &[
    ("L2 + R2", "Alternar entre modo mouse e navegação por itens"),
    (
        "R3 · pressionar analógico direito",
        "Abrir / fechar teclado visual próprio",
    ),
    (
        "L3 · pressionar analógico esquerdo",
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
    ("L3", "Inserir espaço"),
    ("L1", "Inserir Tab"),
    ("R1", "Ativar ou desativar Shift"),
    ("R3", "Fechar o teclado visual"),
    (
        "Start / Menu / +",
        "Ajustes do teclado · direcional escolhe e altera",
    ),
];

pub const ITEM_SHORTCUTS: &[(&str, &str)] = &[
    (
        "Direcional / analógico esquerdo",
        "Mover o foco entre itens próximos",
    ),
    ("Esquerdo · X / □ / Y no Switch", "Ativar o item focado"),
    ("Superior · Y / △ / X no Switch", "Cancelar com Escape"),
    (
        "R3 · pressionar analógico direito",
        "Abrir / fechar teclado visual próprio",
    ),
    ("L2 + R2", "Alternar entre modo mouse e navegação por itens"),
];

const MODE_TRIGGER_PRESS: f64 = 0.55;
const MODE_TRIGGER_RELEASE: f64 = 0.25;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationMode {
    #[default]
    Mouse,
    Items,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadFrame {
    pub x: f64,
    pub y: f64,
    pub scroll: f64,
    pub left_trigger: f64,
    pub right_trigger: f64,
    pub buttons: [bool; 13],
}

impl GamepadFrame {
    pub fn pressed(&self, button: Button) -> bool {
        self.buttons[button as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
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

impl ControlSettings {
    pub fn normalized(mut self) -> Self {
        self.speed = self.speed.clamp(100.0, 2400.0);
        self.deadzone = self.deadzone.clamp(0.05, 0.5);
        self
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
    Menu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(i32, i32),
    Scroll(i32),
    Mouse(MouseButton, bool),
    Key(SystemKey),
    Keyboard(KeyboardInput),
    KeyboardChord(KeyboardInput, KeyboardModifiers),
    NextWindow,
    OnScreenKeyboard(bool),
    ModeChanged(NavigationMode),
    ItemDirection(Direction),
    ItemActivate,
    ItemCancel,
}

pub struct MouseController {
    pub keyboard: KeyboardSession,
    pub mode: NavigationMode,
    previous: GamepadFrame,
    connected: bool,
    armed: bool,
    mode_toggle_armed: bool,
    held: [bool; 2],
    remainder: [f64; 3],
    repeat: [f64; 4],
    axis_direction: Option<Direction>,
    axis_repeat: f64,
    system_keyboard_uses_gamepad: bool,
}

impl Default for MouseController {
    fn default() -> Self {
        Self {
            keyboard: KeyboardSession::default(),
            mode: NavigationMode::Mouse,
            previous: GamepadFrame::default(),
            connected: false,
            armed: false,
            mode_toggle_armed: false,
            held: [false; 2],
            remainder: [0.0; 3],
            repeat: [0.0; 4],
            axis_direction: None,
            axis_repeat: 0.0,
            system_keyboard_uses_gamepad: false,
        }
    }
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
        let triggers_released =
            frame.left_trigger < MODE_TRIGGER_RELEASE && frame.right_trigger < MODE_TRIGGER_RELEASE;
        if triggers_released {
            self.mode_toggle_armed = true;
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
            self.mode_toggle_armed = triggers_released;
            self.previous = *frame;
            return actions;
        }

        let trigger_chord =
            frame.left_trigger >= MODE_TRIGGER_PRESS && frame.right_trigger >= MODE_TRIGGER_PRESS;
        let keyboard_uses_gamepad = self.keyboard.open && self.system_keyboard_uses_gamepad;
        let keyboard_open = self.keyboard.open;
        if trigger_chord && self.mode_toggle_armed {
            self.mode_toggle_armed = false;
            if !keyboard_open {
                self.mode = match self.mode {
                    NavigationMode::Mouse => NavigationMode::Items,
                    NavigationMode::Items => NavigationMode::Mouse,
                };
                self.release(&mut actions);
                actions.push(Action::ModeChanged(self.mode));
                self.previous = *frame;
                return actions;
            }
        }

        let keyboard_toggled =
            frame.pressed(Button::RightStick) && !self.previous.pressed(Button::RightStick);
        if keyboard_toggled {
            actions.push(self.toggle_keyboard());
            self.release(&mut actions);
            self.previous = *frame;
        }
        if keyboard_uses_gamepad && !keyboard_toggled {
            self.release_mouse(&mut actions);
            self.update_keyboard(frame, elapsed, &mut actions);
            self.previous = *frame;
            return actions;
        }

        if self.mode == NavigationMode::Mouse {
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
        }
        let confirm = match self.mode {
            NavigationMode::Mouse => Action::Key(SystemKey::Enter),
            NavigationMode::Items => Action::ItemActivate,
        };
        let cancel = match self.mode {
            NavigationMode::Mouse => Action::Key(SystemKey::Escape),
            NavigationMode::Items => Action::ItemCancel,
        };
        for (button, action) in [
            (Button::West, confirm),
            (Button::North, cancel),
            (Button::LeftStick, Action::Key(SystemKey::Meta)),
            (Button::LeftShoulder, Action::Key(SystemKey::Tab)),
            (Button::RightShoulder, Action::NextWindow),
        ] {
            if !keyboard_toggled && frame.pressed(button) && !self.previous.pressed(button) {
                actions.push(action);
            }
        }
        let seconds = elapsed.as_secs_f64().min(0.05);
        let mut dpad_moved = false;
        let dpad_active = [Button::Up, Button::Down, Button::Left, Button::Right]
            .into_iter()
            .any(|button| frame.pressed(button));
        for (index, (button, direction, key)) in [
            (Button::Up, Direction::Up, SystemKey::Up),
            (Button::Down, Direction::Down, SystemKey::Down),
            (Button::Left, Direction::Left, SystemKey::Left),
            (Button::Right, Direction::Right, SystemKey::Right),
        ]
        .into_iter()
        .enumerate()
        {
            if !keyboard_toggled
                && self.repeat_direction(
                    frame,
                    button,
                    index,
                    elapsed,
                    REPEAT_INITIAL_DELAY,
                    DIRECTION_REPEAT_INTERVAL,
                )
            {
                dpad_moved = true;
                actions.push(if self.mode == NavigationMode::Items {
                    Action::ItemDirection(direction)
                } else {
                    Action::Key(key)
                });
            }
        }
        if self.mode == NavigationMode::Items {
            let axis_direction = dominant_direction(frame.x, frame.y, settings.deadzone);
            if !dpad_active
                && let Some(direction) = axis_direction
                && repeat_axis(
                    &mut self.axis_direction,
                    &mut self.axis_repeat,
                    direction,
                    elapsed,
                )
            {
                actions.push(Action::ItemDirection(direction));
            } else if axis_direction.is_none() || dpad_active || dpad_moved {
                self.axis_direction = None;
                self.axis_repeat = 0.0;
            }
        }
        let x = axis_value(frame.x, settings.deadzone);
        let y = axis_value(frame.y, settings.deadzone);
        let magnitude = x.hypot(y).max(1.0);
        let speed = settings.speed.clamp(100.0, 2400.0);
        if self.mode == NavigationMode::Mouse {
            let dx = accumulate(&mut self.remainder[0], x / magnitude * speed * seconds);
            let dy = accumulate(&mut self.remainder[1], y / magnitude * speed * seconds);
            if dx != 0 || dy != 0 {
                actions.push(Action::Move(dx, dy));
            }
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
        self.axis_direction = None;
        self.axis_repeat = 0.0;
    }

    fn toggle_keyboard(&mut self) -> Action {
        self.keyboard.open = !self.keyboard.open;
        self.keyboard.status = None;
        self.keyboard.customizing = false;
        self.keyboard.modifiers = KeyboardModifiers::default();
        self.keyboard.shift = false;
        Action::OnScreenKeyboard(self.keyboard.open)
    }

    fn close_keyboard(&mut self, actions: &mut Vec<Action>) {
        if self.keyboard.open {
            actions.push(self.toggle_keyboard());
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
        initial_delay: f64,
        repeat_interval: f64,
    ) -> bool {
        if !frame.pressed(button) {
            return false;
        }
        if !self.previous.pressed(button) {
            self.repeat[index] = initial_delay;
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
        if frame.pressed(Button::Start) && !self.previous.pressed(Button::Start) {
            self.keyboard.customizing = !self.keyboard.customizing;
            self.repeat = [f64::from(self.keyboard.settings.repeat_delay_ms) / 1000.0; 4];
            return;
        }
        if self.keyboard.customizing {
            for (button, delta) in [(Button::Up, -1), (Button::Down, 1)] {
                if frame.pressed(button) && !self.previous.pressed(button) {
                    self.keyboard.selected_setting =
                        (self.keyboard.selected_setting as isize + delta).rem_euclid(6) as usize;
                }
            }
            for (button, delta) in [(Button::Left, -1), (Button::Right, 1)] {
                if frame.pressed(button) && !self.previous.pressed(button) {
                    self.keyboard
                        .settings
                        .adjust(self.keyboard.selected_setting, delta);
                }
            }
            return;
        }
        let settings = self.keyboard.settings.normalized();
        for (index, (button, row_delta, column_delta)) in [
            (Button::Up, -1, 0),
            (Button::Down, 1, 0),
            (Button::Left, 0, -1),
            (Button::Right, 0, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let moved = if settings.repeat_enabled {
                self.repeat_direction(
                    frame,
                    button,
                    index,
                    elapsed,
                    f64::from(settings.repeat_delay_ms) / 1000.0,
                    f64::from(settings.repeat_interval_ms) / 1000.0,
                )
            } else {
                frame.pressed(button) && !self.previous.pressed(button)
            };
            if moved {
                self.keyboard.move_selection(row_delta, column_delta);
            }
        }

        for (button, input) in [
            (Button::East, KeyboardInput::Backspace),
            (Button::West, KeyboardInput::Enter),
            (Button::North, KeyboardInput::Escape),
            (Button::LeftStick, KeyboardInput::Space),
            (Button::LeftShoulder, KeyboardInput::Tab),
        ] {
            if frame.pressed(button) && !self.previous.pressed(button) {
                actions.push(self.keyboard_action(input));
            }
        }

        if frame.pressed(Button::RightShoulder) && !self.previous.pressed(Button::RightShoulder) {
            self.keyboard.shift = !self.keyboard.shift;
        }
        if frame.pressed(Button::South) && !self.previous.pressed(Button::South) {
            match self.keyboard.selected_key() {
                key @ KeyboardKey::Character(_) => {
                    let character = key.character(self.keyboard.shifted(key)).unwrap();
                    actions.push(self.keyboard_action(KeyboardInput::Character(character)));
                    self.keyboard.shift = false;
                }
                KeyboardKey::Backspace => {
                    actions.push(self.keyboard_action(KeyboardInput::Backspace))
                }
                KeyboardKey::Enter => actions.push(self.keyboard_action(KeyboardInput::Enter)),
                KeyboardKey::Escape => actions.push(self.keyboard_action(KeyboardInput::Escape)),
                KeyboardKey::Shift => self.keyboard.shift = !self.keyboard.shift,
                KeyboardKey::CapsLock => self.keyboard.caps_lock = !self.keyboard.caps_lock,
                KeyboardKey::Control => {
                    self.keyboard.modifiers.control = !self.keyboard.modifiers.control
                }
                KeyboardKey::Alt => self.keyboard.modifiers.alt = !self.keyboard.modifiers.alt,
                KeyboardKey::Meta => self.keyboard.modifiers.meta = !self.keyboard.modifiers.meta,
                KeyboardKey::Menu => actions.push(self.keyboard_action(KeyboardInput::Menu)),
                KeyboardKey::Space => actions.push(self.keyboard_action(KeyboardInput::Space)),
                KeyboardKey::Tab => actions.push(self.keyboard_action(KeyboardInput::Tab)),
            }
        }
    }

    fn keyboard_action(&mut self, input: KeyboardInput) -> Action {
        let modifiers = std::mem::take(&mut self.keyboard.modifiers);
        if modifiers == KeyboardModifiers::default() {
            Action::Keyboard(input)
        } else {
            Action::KeyboardChord(input, modifiers)
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

fn dominant_direction(x: f64, y: f64, deadzone: f64) -> Option<Direction> {
    let x = axis_value(x, deadzone);
    let y = axis_value(y, deadzone);
    if x.abs().max(y.abs()) < 0.1 {
        None
    } else if x.abs() > y.abs() {
        Some(if x > 0.0 {
            Direction::Right
        } else {
            Direction::Left
        })
    } else {
        Some(if y > 0.0 {
            Direction::Down
        } else {
            Direction::Up
        })
    }
}

fn repeat_axis(
    previous: &mut Option<Direction>,
    remaining: &mut f64,
    direction: Direction,
    elapsed: Duration,
) -> bool {
    if *previous != Some(direction) {
        *previous = Some(direction);
        *remaining = REPEAT_INITIAL_DELAY;
        return true;
    }
    *remaining -= elapsed.as_secs_f64().min(0.05);
    if *remaining > 0.0 {
        return false;
    }
    *remaining += DIRECTION_REPEAT_INTERVAL;
    true
}

fn accumulate(remainder: &mut f64, amount: f64) -> i32 {
    *remainder += amount;
    let whole = remainder.trunc() as i32;
    *remainder -= f64::from(whole);
    whole
}
