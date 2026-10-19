use enigo::{Axis as ScrollAxis, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use crate::control::{Action, KeyboardInput, MouseButton, SystemKey};
use crate::keyboard::{KeyboardModifiers, KeyboardPresentation, KeyboardSession};

pub struct Platform {
    input: Enigo,
}

impl Platform {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            input: Enigo::new(&Settings::default())
                .map_err(|error| format!("Falha ao conectar entrada do Windows: {error}"))?,
        })
    }

    pub fn system_keyboard_uses_gamepad(&self) -> bool {
        true
    }

    pub fn keyboard_presentation(&self) -> KeyboardPresentation {
        KeyboardPresentation::Viewport
    }

    pub fn keyboard_overlay_error(&self) -> Option<&str> {
        None
    }

    pub fn update_keyboard_overlay(&mut self, _: &KeyboardSession) -> Result<(), String> {
        Ok(())
    }

    pub fn apply(&mut self, action: Action) -> Result<(), String> {
        let result = match action {
            Action::OnScreenKeyboard(_) => return Ok(()),
            Action::ModeChanged(_)
            | Action::ItemDirection(_)
            | Action::ItemActivate
            | Action::ItemCancel => return Ok(()),
            Action::Move(x, y) => self.input.move_mouse(x, y, Coordinate::Rel),
            Action::Scroll(amount) => self.input.scroll(amount, ScrollAxis::Vertical),
            Action::Mouse(button, pressed) => self.input.button(
                match button {
                    MouseButton::Left => enigo::Button::Left,
                    MouseButton::Right => enigo::Button::Right,
                },
                if pressed {
                    Direction::Press
                } else {
                    Direction::Release
                },
            ),
            Action::Key(key) => self.input.key(
                match key {
                    SystemKey::Enter => Key::Return,
                    SystemKey::Escape => Key::Escape,
                    SystemKey::Up => Key::UpArrow,
                    SystemKey::Down => Key::DownArrow,
                    SystemKey::Left => Key::LeftArrow,
                    SystemKey::Right => Key::RightArrow,
                    SystemKey::Tab => Key::Tab,
                    SystemKey::Meta => Key::Meta,
                },
                Direction::Click,
            ),
            Action::Keyboard(input) => {
                self.emit_keyboard_input(input, KeyboardModifiers::default())
            }
            Action::KeyboardChord(input, modifiers) => self.emit_keyboard_input(input, modifiers),
            Action::NextWindow => {
                let pressed = self.input.key(Key::Alt, Direction::Press);
                let tab = if pressed.is_ok() {
                    self.input.key(Key::Tab, Direction::Click)
                } else {
                    pressed
                };
                let released = self.input.key(Key::Alt, Direction::Release);
                tab.and(released)
            }
        };
        result.map_err(|error| format!("Falha enviando entrada ao Windows: {error}"))
    }

    fn emit_keyboard_input(
        &mut self,
        input: KeyboardInput,
        modifiers: KeyboardModifiers,
    ) -> Result<(), enigo::InputError> {
        let key = match input {
            KeyboardInput::Character(character) => Key::Unicode(character),
            KeyboardInput::Backspace => Key::Backspace,
            KeyboardInput::Enter => Key::Return,
            KeyboardInput::Escape => Key::Escape,
            KeyboardInput::Space => Key::Space,
            KeyboardInput::Tab => Key::Tab,
            KeyboardInput::Menu => Key::Apps,
        };
        let modifiers: Vec<_> = [
            (modifiers.control, Key::Control),
            (modifiers.alt, Key::Alt),
            (modifiers.meta, Key::Meta),
        ]
        .into_iter()
        .filter_map(|(enabled, key)| enabled.then_some(key))
        .collect();
        let mut result = Ok(());
        for modifier in &modifiers {
            result = result.and_then(|()| self.input.key(*modifier, Direction::Press));
        }
        result = result.and_then(|()| self.input.key(key, Direction::Click));
        for modifier in modifiers.iter().rev() {
            let released = self.input.key(*modifier, Direction::Release);
            result = result.and(released);
        }
        result
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        let _ = self.input.button(enigo::Button::Left, Direction::Release);
        let _ = self.input.button(enigo::Button::Right, Direction::Release);
    }
}
