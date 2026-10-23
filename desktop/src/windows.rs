use std::time::{Duration, Instant};

use enigo::{Axis as ScrollAxis, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use crate::control::{Action, DpadDirection, KeyboardInput, MouseButton, SystemKey};
use crate::keyboard::{KeyboardModifiers, KeyboardPresentation, KeyboardSession};
use crate::windows_navigation::{
    NativeDpadRouting, NativeDpadTargetCache, is_native_windows_target,
};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND},
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            GA_ROOT, GUITHREADINFO, GetAncestor, GetClassNameW, GetForegroundWindow,
            GetGUIThreadInfo, GetWindowThreadProcessId,
        },
    },
    core::PWSTR,
};

const NAVIGATION_TARGET_CHECK_INTERVAL: Duration = Duration::from_millis(50);

pub struct Platform {
    input: Enigo,
    native_dpad: NativeDpadRouting,
    target_cache: NativeDpadTargetCache,
    last_target_check: Option<Instant>,
}

impl Platform {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            input: Enigo::new(&Settings::default())
                .map_err(|error| format!("Falha ao conectar entrada do Windows: {error}"))?,
            native_dpad: NativeDpadRouting::default(),
            target_cache: NativeDpadTargetCache::default(),
            last_target_check: None,
        })
    }

    pub(crate) fn update_native_dpad_target(&mut self) {
        self.refresh_native_dpad_target(false);
    }

    pub(crate) fn take_native_dpad_notice(&mut self) -> bool {
        self.native_dpad.take_notice()
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

    fn refresh_native_dpad_target(&mut self, force: bool) {
        if !force
            && self
                .last_target_check
                .is_some_and(|last| last.elapsed() < NAVIGATION_TARGET_CHECK_INTERVAL)
        {
            return;
        }
        self.last_target_check = Some(Instant::now());

        let Some(window) = Self::focused_root_window() else {
            self.target_cache = NativeDpadTargetCache::default();
            self.native_dpad.set_target_active(false);
            return;
        };
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut process_id));
        }
        let identity = (window.0 as isize, process_id);
        let active = self
            .target_cache
            .resolve(identity, || Self::is_native_system_window(window));
        self.native_dpad.set_target_active(active);
    }

    pub fn apply(&mut self, action: Action) -> Result<(), String> {
        let result = match action {
            Action::OnScreenKeyboard(_) => return Ok(()),
            Action::Dpad(direction) => {
                self.refresh_native_dpad_target(true);
                if !self.native_dpad.should_synthesize() {
                    return Ok(());
                }
                self.input.key(
                    match direction {
                        DpadDirection::Up => Key::UpArrow,
                        DpadDirection::Down => Key::DownArrow,
                        DpadDirection::Left => Key::LeftArrow,
                        DpadDirection::Right => Key::RightArrow,
                    },
                    Direction::Click,
                )
            }
            Action::DpadReleased => {
                self.native_dpad.release_dpad();
                return Ok(());
            }
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

    fn is_native_system_window(window: HWND) -> Option<bool> {
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut process_id));
        }
        if process_id == 0 {
            return None;
        }

        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };
        let mut path = [0_u16; 32_768];
        let mut length = path.len() as u32;
        let result = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut length,
            )
        };
        let _ = unsafe { CloseHandle(process) };
        result.ok()?;

        let windows_directory = std::env::var("WINDIR").ok()?;
        let class = Self::window_class(window);
        if class.is_empty() {
            return None;
        }
        let image_path = String::from_utf16_lossy(&path[..length as usize]);
        Some(is_native_windows_target(
            &image_path,
            &windows_directory,
            &class,
        ))
    }

    fn window_class(window: HWND) -> String {
        let mut name = [0_u16; 128];
        let length = unsafe { GetClassNameW(window, &mut name) };
        if length <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&name[..length as usize])
    }

    fn focused_root_window() -> Option<HWND> {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.is_invalid() {
                return None;
            }

            let mut thread = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            let focus = GetGUIThreadInfo(0, &mut thread)
                .ok()
                .map(|()| thread.hwndFocus)
                .filter(|window| !window.is_invalid())
                .unwrap_or(foreground);
            let root = GetAncestor(focus, GA_ROOT);
            Some(if root.is_invalid() { foreground } else { root })
        }
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        let _ = self.input.button(enigo::Button::Left, Direction::Release);
        let _ = self.input.button(enigo::Button::Right, Direction::Release);
    }
}
