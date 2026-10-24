#[derive(Default)]
pub(crate) struct PausedCursor {
    hidden: bool,
}

impl PausedCursor {
    pub(crate) fn sync<E>(
        &mut self,
        enabled: bool,
        connected: bool,
        mut apply: impl FnMut(bool) -> Result<(), E>,
    ) -> Result<(), E> {
        let hidden = !enabled && connected;
        if self.hidden != hidden {
            // A failed hide can have changed some system cursors already.
            self.hidden = true;
            apply(hidden)?;
            self.hidden = hidden;
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct NativeDpadTargetCache {
    resolved: Option<((isize, u32), bool)>,
}

impl NativeDpadTargetCache {
    pub(crate) fn resolve(
        &mut self,
        identity: (isize, u32),
        identify: impl FnOnce() -> Option<bool>,
    ) -> bool {
        if let Some((cached_identity, active)) = self.resolved
            && cached_identity == identity
        {
            return active;
        }
        let active = identify();
        self.resolved = active.map(|active| (identity, active));
        active.unwrap_or(false)
    }
}

pub(crate) fn resolve_native_windows(
    caches: &mut [NativeDpadTargetCache; 3],
    windows: [(isize, u32); 3],
    mut identify: impl FnMut(isize) -> Option<bool>,
) -> bool {
    caches.iter_mut().zip(windows).any(|(cache, identity)| {
        identity.0 != 0 && cache.resolve(identity, || identify(identity.0))
    })
}

#[derive(Default)]
pub(crate) struct NativeDpadRouting {
    target_active: bool,
    dpad_latched: bool,
}

impl NativeDpadRouting {
    pub(crate) fn set_target_active(&mut self, active: bool) {
        self.target_active = active;
    }

    pub(crate) fn should_synthesize(&mut self) -> bool {
        if self.target_active {
            self.dpad_latched = true;
            false
        } else {
            !self.dpad_latched
        }
    }

    pub(crate) fn release_dpad(&mut self) {
        self.dpad_latched = false;
    }
}

pub(crate) fn is_native_windows_target(
    process_image_path: &str,
    windows_directory: &str,
    window_class: &str,
) -> bool {
    let image_path = normalize_path(process_image_path);
    let windows_directory = normalize_path(windows_directory);
    if image_path.is_empty() || windows_directory.is_empty() {
        return false;
    }

    if image_path.ends_with("/playnite.fullscreenapp.exe") {
        return true;
    }

    if is_child_path(&image_path, &format!("{windows_directory}/systemapps"))
        || is_child_path(
            &image_path,
            &format!("{windows_directory}/immersivecontrolpanel"),
        )
        || image_path == format!("{windows_directory}/system32/shellhost.exe")
            && window_class.eq_ignore_ascii_case("ControlCenterWindow")
    {
        return true;
    }

    image_path == format!("{windows_directory}/explorer.exe")
        && matches!(
            window_class.to_ascii_lowercase().as_str(),
            "shell_traywnd"
                | "shell_secondarytraywnd"
                | "notifyiconoverflowwindow"
                | "xamlexplorerhostislandwindow"
                | "windows.ui.core.corewindow"
                | "windows.ui.composition.desktopwindowcontentbridge"
        )
}

fn normalize_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_child_path(path: &str, directory: &str) -> bool {
    path.strip_prefix(directory)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_settings_shellhost_does_not_receive_duplicate_directional_repeats() {
        for (process, windows_directory) in [
            (r"C:\Windows\System32\ShellHost.exe", r"C:\Windows"),
            ("d:/WINDOWS/system32/SHELLHOST.EXE", "D:/Windows/"),
        ] {
            let mut routing = NativeDpadRouting::default();
            routing.set_target_active(is_native_windows_target(
                process,
                windows_directory,
                "ControlCenterWindow",
            ));
            for _ in 0..10 {
                assert!(
                    !routing.should_synthesize(),
                    "duplicate arrow for {process}"
                );
            }
            routing.release_dpad();
            assert!(!routing.should_synthesize());
            routing.release_dpad();
            routing.set_target_active(false);
            assert!(
                routing.should_synthesize(),
                "ordinary apps still need arrows"
            );
        }
    }

    #[test]
    fn shellhost_detection_requires_the_system_path_and_panel_class() {
        for (process, class) in [
            (r"C:\Other\ShellHost.exe", "ControlCenterWindow"),
            (r"C:\Windows\System32\Other.exe", "ControlCenterWindow"),
            (r"C:\Windows\System32\ShellHost.exe", "OtherWindow"),
        ] {
            assert!(!is_native_windows_target(process, r"C:\Windows", class));
        }
    }

    #[test]
    fn native_focused_panel_or_foreground_wins_over_non_native_root() {
        for native in [11, 22, 33] {
            let mut caches = Default::default();
            let windows = [(11, 1), (22, 2), (33, 3)];
            let mut routing = NativeDpadRouting::default();
            routing.set_target_active(resolve_native_windows(&mut caches, windows, |handle| {
                Some(handle == native)
            }));
            assert!(
                !routing.should_synthesize(),
                "native surface {native} was missed"
            );
            assert!(!routing.should_synthesize());
            routing.release_dpad();
            assert!(!routing.should_synthesize());
        }
    }

    #[test]
    fn paused_cursor_hides_once_and_restores_on_resume_or_disconnect() {
        let mut cursor = PausedCursor::default();
        let mut changes = Vec::new();
        for (enabled, connected) in [
            (true, true),
            (false, true),
            (false, true),
            (true, true),
            (false, true),
            (false, false),
            (false, false),
        ] {
            cursor
                .sync(enabled, connected, |hidden| {
                    changes.push(hidden);
                    Ok::<_, ()>(())
                })
                .unwrap();
        }
        assert_eq!(changes, [true, false, true, false]);
    }

    #[test]
    fn failed_cursor_hide_still_requires_restoration() {
        let mut cursor = PausedCursor::default();
        assert!(cursor.sync(false, true, |_| Err("partial hide")).is_err());
        let mut restored = false;
        cursor
            .sync(true, false, |hidden| {
                restored = !hidden;
                Ok::<_, ()>(())
            })
            .unwrap();
        assert!(restored);
    }

    #[test]
    fn failed_cursor_restore_can_be_retried() {
        let mut cursor = PausedCursor::default();
        cursor.sync(false, true, |_| Ok::<_, ()>(())).unwrap();
        assert!(cursor.sync(true, true, |_| Err(())).is_err());
        let mut retried = false;
        cursor
            .sync(true, true, |hidden| {
                retried = !hidden;
                Ok::<_, ()>(())
            })
            .unwrap();
        assert!(retried);
    }

    #[test]
    fn failed_target_lookup_is_retried_and_then_stops_synthetic_dpad() {
        let mut cache = NativeDpadTargetCache::default();
        let mut routing = NativeDpadRouting::default();
        let identity = (17, 42);

        routing.set_target_active(cache.resolve(identity, || None));
        assert!(routing.should_synthesize());

        routing.set_target_active(cache.resolve(identity, || Some(true)));
        assert!(!routing.should_synthesize());
    }

    #[test]
    fn resolved_targets_are_cached_and_window_changes_trigger_lookup() {
        let mut cache = NativeDpadTargetCache::default();
        assert!(!cache.resolve((17, 42), || Some(false)));
        assert!(!cache.resolve((17, 42), || panic!("resolved target should be cached")));
        assert!(cache.resolve((18, 42), || Some(true)));
        assert!(cache.resolve((18, 42), || panic!("resolved target should be cached")));
    }

    #[test]
    fn recognizes_windows_system_app_surfaces_but_not_other_packaged_apps() {
        assert!(is_native_windows_target(
            r"C:\Windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy\ShellExperienceHost.exe",
            r"C:\Windows",
            "Windows.UI.Core.CoreWindow",
        ));
        assert!(is_native_windows_target(
            r"C:\Windows\ImmersiveControlPanel\SystemSettings.exe",
            r"C:\Windows",
            "ApplicationFrameWindow",
        ));
        assert!(!is_native_windows_target(
            r"C:\Program Files\WindowsApps\Contoso.App\App.exe",
            r"C:\Windows",
            "ApplicationFrameWindow",
        ));
    }

    #[test]
    fn recognizes_taskbar_windows_without_classifying_file_explorer() {
        assert!(is_native_windows_target(
            r"C:\Windows\explorer.exe",
            r"C:\Windows",
            "Shell_TrayWnd",
        ));
        assert!(!is_native_windows_target(
            r"C:\Windows\explorer.exe",
            r"C:\Windows",
            "CabinetWClass",
        ));
    }

    #[test]
    fn native_dpad_suppresses_synthetic_repeats_until_release() {
        let mut routing = NativeDpadRouting::default();
        routing.set_target_active(true);

        assert!(!routing.should_synthesize());
        assert!(!routing.should_synthesize());

        routing.set_target_active(false);
        assert!(!routing.should_synthesize());
        routing.release_dpad();
        assert!(routing.should_synthesize());
    }

    #[test]
    fn native_navigation_suppression_recovers_after_reentering_the_system_ui() {
        let mut routing = NativeDpadRouting::default();
        routing.set_target_active(true);
        assert!(!routing.should_synthesize());
        routing.release_dpad();

        routing.set_target_active(false);
        routing.set_target_active(true);
        assert!(!routing.should_synthesize());
    }
}
