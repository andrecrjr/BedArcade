#[cfg(target_os = "linux")]
pub(crate) fn set_enabled(enabled: bool) -> Result<(), String> {
    use std::{fs, io::ErrorKind, path::PathBuf};

    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let Some(path) = config_dir.map(|path| path.join("autostart/gamepad-desktop.desktop")) else {
        return if enabled {
            Err("Configuration folder is unavailable".to_owned())
        } else {
            Ok(())
        };
    };

    if !enabled {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        };
    }

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable = executable
        .to_str()
        .ok_or_else(|| "Executable path is not valid UTF-8".to_owned())?;
    if executable.chars().any(char::is_control) {
        return Err("Executable path contains unsupported characters".to_owned());
    }
    let directory = path
        .parent()
        .ok_or_else(|| "Autostart folder is unavailable".to_owned())?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Gamepad Desktop\nExec={} --background\nTerminal=false\nNoDisplay=true\nStartupNotify=false\n",
        desktop_exec_argument(executable)
    );
    fs::write(path, entry).map_err(|error| error.to_string())
}

#[cfg(target_os = "linux")]
fn desktop_exec_argument(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
            .replace('%', "%%")
    )
}

#[cfg(target_os = "windows")]
pub(crate) fn set_enabled(enabled: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    use windows::{
        Win32::{
            Foundation::ERROR_FILE_NOT_FOUND,
            System::Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
                RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
            },
        },
        core::{PCWSTR, w},
    };

    let mut key = HKEY(std::ptr::null_mut());
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    }
    .ok()
    .map_err(|error| error.to_string())?;

    let result = if enabled {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let mut command = vec![b'"' as u16];
        command.extend(executable.as_os_str().encode_wide());
        command.extend("\" --background".encode_utf16());
        command.push(0);
        let value = command
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect::<Vec<_>>();
        unsafe { RegSetValueExW(key, w!("GamepadDesktop"), None, REG_SZ, Some(&value)) }
            .ok()
            .map_err(|error| error.to_string())
    } else {
        let status = unsafe { RegDeleteValueW(key, w!("GamepadDesktop")) };
        if status.0 == ERROR_FILE_NOT_FOUND.0 {
            Ok(())
        } else {
            status.ok().map_err(|error| error.to_string())
        }
    };
    let close = unsafe { RegCloseKey(key) }
        .ok()
        .map_err(|error| error.to_string());
    result.and(close)
}
