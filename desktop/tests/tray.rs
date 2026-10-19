use eframe::egui::{Context, RawInput, ViewportCommand, ViewportId};
use gamepad_desktop::tray::{PanelControl, TrayAction};

#[test]
fn closed_panel_can_be_reopened_without_exiting() {
    let (panel, receiver) = PanelControl::new();
    panel.attach(Some(Context::default()));
    panel.attach(None);
    panel.show();
    assert_eq!(receiver.try_recv().unwrap(), TrayAction::Open);
    assert!(!panel.exiting());
}

#[test]
fn opening_an_existing_panel_restores_and_focuses_it() {
    let (panel, receiver) = PanelControl::new();
    let context = Context::default();
    let mut output = context.run_pass(RawInput::default(), |context| {
        panel.attach(Some(context.clone()));
        panel.show();
    });
    output.textures_delta.clear();
    let commands = &output.viewport_output[&ViewportId::ROOT].commands;
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, ViewportCommand::Visible(true)))
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, ViewportCommand::Minimized(false)))
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, ViewportCommand::Focus))
    );
    assert!(receiver.try_recv().is_err());
}

#[test]
fn explicit_exit_closes_panel_and_wakes_the_background_loop() {
    let (panel, receiver) = PanelControl::new();
    let context = Context::default();
    let mut output = context.run_pass(RawInput::default(), |context| {
        panel.attach(Some(context.clone()));
        panel.exit();
    });
    output.textures_delta.clear();
    assert!(panel.exiting());
    assert_eq!(receiver.try_recv().unwrap(), TrayAction::Exit);
    assert!(
        output.viewport_output[&ViewportId::ROOT]
            .commands
            .iter()
            .any(|command| matches!(command, ViewportCommand::Close))
    );
}

#[test]
fn hiding_panel_requests_window_close_without_exiting_the_service() {
    let (panel, receiver) = PanelControl::new();
    let context = Context::default();
    let mut output = context.run_pass(RawInput::default(), |context| {
        panel.attach(Some(context.clone()));
        panel.hide();
    });
    output.textures_delta.clear();
    assert!(!panel.exiting());
    assert!(receiver.try_recv().is_err());
    assert!(
        output.viewport_output[&ViewportId::ROOT]
            .commands
            .iter()
            .any(|command| matches!(command, ViewportCommand::Close))
    );
}
