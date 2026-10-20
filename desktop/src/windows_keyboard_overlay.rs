use std::sync::atomic::{AtomicBool, Ordering};

use windows::{
    Win32::{
        Foundation::{FALSE, HWND, LPARAM, TRUE},
        System::Threading::GetCurrentProcessId,
        UI::WindowsAndMessaging::{
            EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId,
            SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW,
            SetWindowPos, WS_EX_LAYERED, WS_EX_TRANSPARENT,
        },
    },
    core::BOOL,
};

const KEYBOARD_TITLE: &str = "Gamepad Navigator · Teclado";
static CLICK_THROUGH_APPLIED: AtomicBool = AtomicBool::new(false);

pub fn enable_click_through() {
    if CLICK_THROUGH_APPLIED.load(Ordering::Acquire) {
        return;
    }

    let mut search = WindowSearch {
        process_id: unsafe { GetCurrentProcessId() },
        keyboard: None,
    };
    if unsafe {
        EnumWindows(
            Some(find_keyboard_window),
            LPARAM((&mut search as *mut WindowSearch) as isize),
        )
    }
    .is_err()
    {
        return;
    }
    let Some(hwnd) = search.keyboard else {
        return;
    };

    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let click_through = click_through_style(current as u32) as isize;
        if current != click_through {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, click_through);
            if SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
            )
            .is_err()
            {
                return;
            }
        }
    }
    CLICK_THROUGH_APPLIED.store(true, Ordering::Release);
}

fn click_through_style(style: u32) -> u32 {
    (style | WS_EX_TRANSPARENT.0) & !WS_EX_LAYERED.0
}

struct WindowSearch {
    process_id: u32,
    keyboard: Option<HWND>,
}

unsafe extern "system" fn find_keyboard_window(hwnd: HWND, data: LPARAM) -> BOOL {
    let search = unsafe { &mut *(data.0 as *mut WindowSearch) };
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id != search.process_id {
        return TRUE;
    }

    let mut title = [0u16; 64];
    let length = unsafe { GetWindowTextW(hwnd, &mut title) };
    if length > 0
        && KEYBOARD_TITLE
            .encode_utf16()
            .eq(title[..length as usize].iter().copied())
    {
        search.keyboard = Some(hwnd);
        FALSE
    } else {
        TRUE
    }
}
