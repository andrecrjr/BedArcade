use gamepad_desktop::gamepads::{GamepadInfo, save_mapping, select_device, validate_mapping};

fn device(id: u32, name: &str, mapped: bool) -> GamepadInfo {
    GamepadInfo {
        id,
        name: name.to_owned(),
        guid: format!("{id:032x}"),
        mapped,
    }
}

#[test]
fn selection_accepts_different_brands_and_respects_the_chosen_controller() {
    let devices = vec![
        device(1, "Unknown USB Gamepad", false),
        device(2, "Xbox", true),
        device(3, "DualSense", true),
        device(4, "Switch Pro", true),
        device(5, "Generic HID", true),
    ];
    assert_eq!(select_device(&devices, None), Some(2));
    for id in 2..=5 {
        assert_eq!(select_device(&devices, Some(id)), Some(id));
    }
    assert_eq!(select_device(&devices, Some(1)), None);
    assert_eq!(select_device(&devices, Some(99)), None);
}

#[test]
fn disconnected_selection_does_not_silently_control_another_gamepad() {
    let devices = vec![device(2, "Other controller", true)];
    assert_eq!(select_device(&devices, Some(1)), None);
    assert_eq!(select_device(&devices, None), Some(2));
}

#[test]
fn custom_profiles_require_one_valid_sdl_mapping_line() {
    let line = "03000000000000000000000000000001,Generic Pad,a:b0,leftx:a0,lefty:a1,";
    assert_eq!(validate_mapping(line).unwrap(), line);
    for invalid in [
        "",
        "not-a-guid,Pad,a:b0,",
        "03000000000000000000000000000001,,a:b0,",
        "03000000000000000000000000000001,Pad",
        "03000000000000000000000000000001,Pad,a:b0,\nsecond",
        "03000000000000000000000000000001,Pad,a:b0,\0",
    ] {
        assert!(
            validate_mapping(invalid).is_err(),
            "must reject {invalid:?}"
        );
    }
}

#[test]
fn saving_a_profile_replaces_its_guid_and_preserves_other_controllers() {
    let directory = std::env::temp_dir().join(format!(
        "gamepad-profiles-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = directory.join("profiles/gamecontrollerdb.txt");
    let first = "03000000000000000000000000000001,Pad One,a:b0,";
    let other = "03000000000000000000000000000002,Pad Two,a:b1,";
    let updated = "03000000000000000000000000000001,Pad One,a:b3,";
    save_mapping(&path, first).unwrap();
    save_mapping(&path, other).unwrap();
    save_mapping(&path, updated).unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents.lines().count(), 2);
    assert!(contents.contains(other));
    assert!(contents.contains(updated));
    assert!(!contents.contains(first));
    assert!(save_mapping(&path, "invalid").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
    std::fs::remove_dir_all(directory).unwrap();
}
