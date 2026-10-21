use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
};

use crate::i18n::{Locale, Text};
use eframe::egui::{Context, IconData, ViewportCommand};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Open,
    Exit,
}

pub struct PanelControl {
    context: Mutex<Option<Context>>,
    sender: Sender<TrayAction>,
    exit_requested: AtomicBool,
}

#[derive(Clone)]
pub struct TrayMenu {
    open: MenuItem,
    hide: MenuItem,
    exit: MenuItem,
}

impl TrayMenu {
    pub fn set_locale(&self, locale: Locale) {
        self.open.set_text(locale.text(Text::TrayOpen));
        self.hide.set_text(locale.text(Text::TrayHide));
        self.exit.set_text(locale.text(Text::QuitApplication));
    }
}

impl PanelControl {
    pub fn new() -> (Arc<Self>, Receiver<TrayAction>) {
        let (sender, receiver) = mpsc::channel();
        (
            Arc::new(Self {
                context: Mutex::new(None),
                sender,
                exit_requested: AtomicBool::new(false),
            }),
            receiver,
        )
    }

    pub fn attach(&self, context: Option<Context>) {
        *self.context.lock().expect("panel context lock") = context;
    }

    pub fn exiting(&self) -> bool {
        self.exit_requested.load(Ordering::Relaxed)
    }

    pub fn show(&self) {
        let context = self.context.lock().expect("panel context lock").clone();
        if let Some(context) = context {
            context.send_viewport_cmd(ViewportCommand::Visible(true));
            context.send_viewport_cmd(ViewportCommand::Minimized(false));
            context.send_viewport_cmd(ViewportCommand::Focus);
            context.request_repaint();
        } else {
            let _ = self.sender.send(TrayAction::Open);
        }
    }

    pub fn hide(&self) {
        if let Some(context) = self.context.lock().expect("panel context lock").as_ref() {
            context.send_viewport_cmd(ViewportCommand::Close);
            context.request_repaint();
        }
    }

    pub fn exit(&self) {
        self.exit_requested.store(true, Ordering::Relaxed);
        let _ = self.sender.send(TrayAction::Exit);
        self.hide();
    }
}

pub fn create_tray(
    panel: Arc<PanelControl>,
    locale: Locale,
) -> Result<(TrayIcon, TrayMenu), String> {
    let menu = Menu::new();
    let open = MenuItem::new(locale.text(Text::TrayOpen), true, None);
    let hide = MenuItem::new(locale.text(Text::TrayHide), true, None);
    let exit = MenuItem::new(locale.text(Text::QuitApplication), true, None);
    let labels = TrayMenu {
        open: open.clone(),
        hide: hide.clone(),
        exit: exit.clone(),
    };
    menu.append_items(&[&open, &hide, &PredefinedMenuItem::separator(), &exit])
        .map_err(|error| error.to_string())?;
    let (open_id, hide_id, exit_id) = (open.id().clone(), hide.id().clone(), exit.id().clone());
    let menu_panel = Arc::clone(&panel);
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == open_id {
            menu_panel.show();
        } else if event.id == hide_id {
            menu_panel.hide();
        } else if event.id == exit_id {
            menu_panel.exit();
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
        ) {
            panel.show();
        }
    }));
    let icon = TrayIconBuilder::new()
        .with_id("gamepad-desktop")
        .with_tooltip("Gamepad Desktop")
        .with_title("Gamepad Desktop")
        .with_icon(gamepad_icon()?)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| error.to_string())?;
    Ok((icon, labels))
}

fn gamepad_icon() -> Result<Icon, String> {
    let icon = gamepad_icon_data(32);
    Icon::from_rgba(icon.rgba, icon.width, icon.height).map_err(|error| error.to_string())
}

pub fn gamepad_icon_data(size: u32) -> IconData {
    let image = image::load_from_memory(include_bytes!("../assets/gamepad-desktop.png"))
        .expect("embedded desktop icon is a valid PNG")
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    IconData {
        rgba: image.into_raw(),
        width: size,
        height: size,
    }
}
