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

#[derive(Default)]
pub(crate) struct NativeDpadRouting {
    target_active: bool,
    dpad_latched: bool,
    notice_sent: bool,
    notice_pending: bool,
}

impl NativeDpadRouting {
    pub(crate) fn set_target_active(&mut self, active: bool) {
        self.target_active = active;
        if !active {
            self.notice_sent = false;
        }
    }

    pub(crate) fn should_synthesize(&mut self) -> bool {
        if self.target_active {
            if !self.notice_sent {
                self.notice_pending = true;
                self.notice_sent = true;
            }
            self.dpad_latched = true;
            false
        } else {
            !self.dpad_latched
        }
    }

    pub(crate) fn release_dpad(&mut self) {
        self.dpad_latched = false;
    }

    pub(crate) fn take_notice(&mut self) -> bool {
        std::mem::take(&mut self.notice_pending)
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

    if is_child_path(&image_path, &format!("{windows_directory}/systemapps"))
        || is_child_path(
            &image_path,
            &format!("{windows_directory}/immersivecontrolpanel"),
        )
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
    fn failed_target_lookup_is_retried_and_then_stops_synthetic_dpad() {
        let mut cache = NativeDpadTargetCache::default();
        let mut routing = NativeDpadRouting::default();
        let identity = (17, 42);

        routing.set_target_active(cache.resolve(identity, || None));
        assert!(routing.should_synthesize());

        routing.set_target_active(cache.resolve(identity, || Some(true)));
        assert!(!routing.should_synthesize());
        assert!(routing.take_notice());
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
        assert!(routing.take_notice());
        assert!(!routing.should_synthesize());
        assert!(!routing.take_notice());

        routing.set_target_active(false);
        assert!(!routing.should_synthesize());
        routing.release_dpad();
        assert!(routing.should_synthesize());
    }

    #[test]
    fn native_navigation_notice_can_be_sent_again_after_leaving_the_system_ui() {
        let mut routing = NativeDpadRouting::default();
        routing.set_target_active(true);
        assert!(!routing.should_synthesize());
        assert!(routing.take_notice());
        routing.release_dpad();

        routing.set_target_active(false);
        routing.set_target_active(true);
        assert!(!routing.should_synthesize());
        assert!(routing.take_notice());
    }
}
