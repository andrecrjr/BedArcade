use std::{thread, time::Duration};

use evdev::uinput::VirtualDevice;

use super::*;

#[test]
fn every_qwerty_character_has_a_linux_key_mapping() {
    for row in crate::keyboard::KEY_ROWS {
        for key in *row {
            for shifted in [false, true] {
                if let Some(character) = key.character(shifted) {
                    assert!(
                        ascii_key(character).is_some(),
                        "unsupported keycap: {character}"
                    );
                }
            }
        }
    }
}

fn node(device: &mut VirtualDevice) -> std::path::PathBuf {
    device
        .enumerate_dev_nodes_blocking()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
}

#[test]
#[ignore = "requires /dev/uinput and udevadm"]
fn virtual_devices_are_classified_as_mouse_and_keyboard() {
    let mut platform = Platform::new().unwrap();
    for (path, expected) in [
        (node(&mut platform.pointer), "ID_INPUT_MOUSE=1"),
        (node(&mut platform.keyboard), "ID_INPUT_KEYBOARD=1"),
    ] {
        let mut classified = false;
        for _ in 0..80 {
            let output = std::process::Command::new("udevadm")
                .args(["info", "--query=property", "--name"])
                .arg(&path)
                .output()
                .unwrap();
            if output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line == expected)
            {
                classified = true;
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        assert!(
            classified,
            "{path:?} must have {expected} so the compositor accepts it"
        );
    }
}
