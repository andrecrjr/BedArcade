use enigo::{Axis as ScrollAxis, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use crate::control::{Action, KeyboardInput, MouseButton, SystemKey};
use crate::keyboard::{KeyboardPresentation, KeyboardSession};

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
            Action::Keyboard(input) => self.input.key(
                match input {
                    KeyboardInput::Character(character) => Key::Unicode(character),
                    KeyboardInput::Backspace => Key::Backspace,
                    KeyboardInput::Enter => Key::Return,
                    KeyboardInput::Escape => Key::Escape,
                    KeyboardInput::Space => Key::Space,
                    KeyboardInput::Tab => Key::Tab,
                },
                Direction::Click,
            ),
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
}

impl Drop for Platform {
    fn drop(&mut self) {
        let _ = self.input.button(enigo::Button::Left, Direction::Release);
        let _ = self.input.button(enigo::Button::Right, Direction::Release);
    }
}
