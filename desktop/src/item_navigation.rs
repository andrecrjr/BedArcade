use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};

use crate::{control::Direction, placement::Rect};

const MAX_ITEMS: usize = 2_000;

#[derive(Clone, Copy, Debug)]
enum Command {
    Enable(bool),
    Move(Direction),
    Activate,
    Refresh,
    Stop,
}

pub struct ItemNavigator {
    sender: Sender<Command>,
    status: Arc<Mutex<String>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Default for ItemNavigator {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(Mutex::new(
            "Ative o modo itens para consultar a árvore de acessibilidade.".to_owned(),
        ));
        let worker_status = Arc::clone(&status);
        let worker = std::thread::spawn(move || run(receiver, worker_status));
        Self {
            sender,
            status,
            worker: Some(worker),
        }
    }
}

impl ItemNavigator {
    pub fn set_enabled(&self, enabled: bool) {
        let _ = self.sender.send(Command::Enable(enabled));
    }

    pub fn move_focus(&self, direction: Direction) {
        let _ = self.sender.send(Command::Move(direction));
    }

    pub fn activate(&self) {
        let _ = self.sender.send(Command::Activate);
    }

    pub fn refresh(&self) {
        let _ = self.sender.send(Command::Refresh);
    }

    pub fn status(&self) -> String {
        self.status.lock().expect("item navigation status").clone()
    }
}

impl Drop for ItemNavigator {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn set_status(status: &Mutex<String>, message: impl Into<String>) {
    *status.lock().expect("item navigation status") = message.into();
}

pub fn nearest_neighbor(
    bounds: &[Rect],
    current: Option<usize>,
    direction: Direction,
) -> Option<usize> {
    let current = current.filter(|index| bounds.get(*index).is_some_and(|rect| rect.valid()));
    let Some(current) = current else {
        return bounds
            .iter()
            .enumerate()
            .filter(|(_, rect)| rect.valid())
            .min_by_key(|(_, rect)| (rect.y, rect.x))
            .map(|(index, _)| index);
    };
    let from = center(bounds[current]);
    bounds
        .iter()
        .enumerate()
        .filter(|(index, rect)| {
            *index != current
                && rect.valid()
                && match direction {
                    Direction::Up => center(**rect).1 < from.1,
                    Direction::Down => center(**rect).1 > from.1,
                    Direction::Left => center(**rect).0 < from.0,
                    Direction::Right => center(**rect).0 > from.0,
                }
        })
        .min_by(|(_, left), (_, right)| {
            neighbor_score(from, center(**left), direction).total_cmp(&neighbor_score(
                from,
                center(**right),
                direction,
            ))
        })
        .map(|(index, _)| index)
}

fn center(rect: Rect) -> (f64, f64) {
    (
        f64::from(rect.x) + f64::from(rect.width) / 2.0,
        f64::from(rect.y) + f64::from(rect.height) / 2.0,
    )
}

fn neighbor_score(from: (f64, f64), to: (f64, f64), direction: Direction) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let (forward, perpendicular) = match direction {
        Direction::Up => (-dy, dx),
        Direction::Down => (dy, dx),
        Direction::Left => (-dx, dy),
        Direction::Right => (dx, dy),
    };
    forward * forward + perpendicular * perpendicular * 2.0
}

#[cfg(target_os = "linux")]
fn run(receiver: Receiver<Command>, status: Arc<Mutex<String>>) {
    linux::run(receiver, status);
}

#[cfg(target_os = "windows")]
fn run(receiver: Receiver<Command>, status: Arc<Mutex<String>>) {
    windows::run(receiver, status);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32) -> Rect {
        Rect {
            x,
            y,
            width: 20,
            height: 20,
        }
    }

    #[test]
    fn directional_neighbor_uses_the_nearest_item_ahead_and_does_not_wrap() {
        let items = [rect(0, 0), rect(0, 40), rect(50, 20), rect(0, -40)];
        assert_eq!(nearest_neighbor(&items, Some(0), Direction::Down), Some(1));
        assert_eq!(nearest_neighbor(&items, Some(0), Direction::Right), Some(2));
        assert_eq!(nearest_neighbor(&items, Some(0), Direction::Up), Some(3));
        assert_eq!(nearest_neighbor(&items, Some(0), Direction::Left), None);
    }

    #[test]
    fn first_focus_skips_invalid_bounds_and_starts_at_top_left() {
        let items = [
            Rect {
                width: -1,
                ..rect(-20, -20)
            },
            rect(50, 0),
            rect(-20, 0),
        ];
        assert_eq!(nearest_neighbor(&items, None, Direction::Right), Some(2));
    }

    #[test]
    fn diagonal_neighbors_prefer_the_closer_axis_aligned_item() {
        let items = [rect(0, 0), rect(0, 60), rect(50, 40)];
        assert_eq!(nearest_neighbor(&items, Some(0), Direction::Down), Some(1));
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use atspi::{
        CoordType, ObjectRefOwned, Role, ScrollType, State,
        proxy::{
            accessible::AccessibleProxy, action::ActionProxy, bus::BusProxy,
            component::ComponentProxy,
        },
    };
    use futures_lite::future::block_on;
    use zbus::{Connection, connection::Builder};

    struct Candidate {
        item: ObjectRefOwned,
        bounds: Rect,
        name: String,
        focused: bool,
    }

    struct Navigator {
        bus: Connection,
        candidates: Vec<Candidate>,
        selected: Option<usize>,
    }

    impl Navigator {
        async fn new() -> zbus::Result<Self> {
            let session = Builder::session()
                .map_err(|error| zbus::Error::Failure(error.to_string()))?
                .method_timeout(std::time::Duration::from_millis(350))
                .build()
                .await?;
            let address = BusProxy::new(&session).await?.get_address().await?;
            let bus = Builder::address(address.as_str())?
                .method_timeout(std::time::Duration::from_millis(350))
                .build()
                .await?;
            Ok(Self {
                bus,
                candidates: Vec::new(),
                selected: None,
            })
        }

        async fn refresh(&mut self, status: &Mutex<String>) -> zbus::Result<()> {
            set_status(status, "Lendo os itens acessíveis da janela ativa…");
            let root = AccessibleProxy::builder(&self.bus)
                .cache_properties(zbus::proxy::CacheProperties::No)
                .destination("org.a11y.atspi.Registry")?
                .path("/org/a11y/atspi/accessible/root")?
                .build()
                .await?;
            let applications = root.get_children().await?;
            let mut active_root = None;
            let mut modal_root = None;
            for application in applications {
                let Some(proxy) = accessible(&self.bus, &application).await else {
                    continue;
                };
                let children = proxy.get_children().await.unwrap_or_default();
                let app_active = proxy
                    .get_state()
                    .await
                    .is_ok_and(|states| states.contains(State::Active));
                let mut focused_window = None;
                let mut modal_window = None;
                for window_item in children {
                    if let Some(window) = accessible(&self.bus, &window_item).await {
                        let states = window.get_state().await.unwrap_or_default();
                        if states.contains(State::Active) || states.contains(State::Focused) {
                            focused_window = Some(window_item);
                            break;
                        }
                        if states.contains(State::Modal) {
                            modal_window = Some(window_item);
                        }
                    }
                }
                if let Some(window) = focused_window {
                    active_root = Some(window);
                    break;
                }
                if app_active {
                    active_root = modal_window.or_else(|| Some(application.clone()));
                    break;
                }
                if modal_root.is_none() {
                    modal_root = modal_window;
                }
            }
            let Some(application) = active_root.or(modal_root) else {
                self.candidates.clear();
                self.selected = None;
                set_status(
                    status,
                    "AT-SPI não identificou uma janela ativa para navegar.",
                );
                return Ok(());
            };
            let mut pending = vec![application];
            let mut candidates = Vec::new();
            let mut scanned = 0;
            while let Some(item) = pending.pop() {
                scanned += 1;
                if candidates.len() >= MAX_ITEMS || scanned > MAX_ITEMS * 4 {
                    break;
                }
                let Some(proxy) = accessible(&self.bus, &item).await else {
                    continue;
                };
                let state = proxy.get_state().await.ok();
                let role = proxy.get_role().await.ok();
                let name = proxy.name().await.unwrap_or_default();
                if let (Some(state), Some(role)) = (state, role)
                    && actionable(role)
                    && (state.contains(State::Enabled) || state.contains(State::Sensitive))
                    && state.contains(State::Visible)
                    && state.contains(State::Showing)
                    && (state.contains(State::Focusable)
                        || state.contains(State::Selectable)
                        || state.contains(State::Editable))
                    && let Some(bounds) = extents(&self.bus, &item).await
                {
                    candidates.push(Candidate {
                        item: item.clone(),
                        bounds,
                        name,
                        focused: state.contains(State::Focused),
                    });
                }
                if let Ok(mut children) = proxy.get_children().await {
                    pending.append(&mut children);
                }
            }
            self.selected = candidates.iter().position(|candidate| candidate.focused);
            if self.selected.is_none() {
                self.selected = nearest_neighbor(
                    &candidates
                        .iter()
                        .map(|candidate| candidate.bounds)
                        .collect::<Vec<_>>(),
                    None,
                    Direction::Right,
                );
                if let Some(index) = self.selected {
                    focus(&self.bus, &candidates[index].item).await;
                }
            }
            self.candidates = candidates;
            if let Some(index) = self.selected {
                set_status(status, item_label(&self.candidates[index].name));
            } else {
                set_status(
                    status,
                    "Esta janela não expõe itens interativos visíveis pela acessibilidade.",
                );
            }
            Ok(())
        }

        async fn move_focus(&mut self, direction: Direction, status: &Mutex<String>) {
            let bounds = self
                .candidates
                .iter()
                .map(|candidate| candidate.bounds)
                .collect::<Vec<_>>();
            let Some(index) = nearest_neighbor(&bounds, self.selected, direction) else {
                set_status(status, "Não há outro item nessa direção.");
                return;
            };
            let item = &self.candidates[index].item;
            if focus(&self.bus, item).await {
                self.selected = Some(index);
                set_status(status, item_label(&self.candidates[index].name));
            } else {
                set_status(status, "O aplicativo não aceitou o foco deste item.");
                let _ = self.refresh(status).await;
            }
        }

        async fn activate(&mut self, status: &Mutex<String>) {
            let Some(index) = self.selected else {
                set_status(status, "Nenhum item está focado para ativar.");
                return;
            };
            let item = &self.candidates[index].item;
            let result = if let Some(proxy) = action(&self.bus, item).await {
                activate_action(&proxy).await
            } else {
                false
            };
            if result {
                set_status(status, "Item ativado.");
                let _ = self.refresh(status).await;
            } else {
                set_status(
                    status,
                    "O item focado não oferece uma ação de ativação AT-SPI.",
                );
            }
        }
    }

    pub fn run(receiver: Receiver<Command>, status: Arc<Mutex<String>>) {
        let mut navigator = match block_on(Navigator::new()) {
            Ok(navigator) => navigator,
            Err(error) => {
                set_status(&status, format!("AT-SPI indisponível: {error}"));
                wait_until_stop(receiver);
                return;
            }
        };
        while let Ok(command) = receiver.recv() {
            match command {
                Command::Enable(true) | Command::Refresh => {
                    if let Err(error) = block_on(navigator.refresh(&status)) {
                        set_status(
                            &status,
                            format!("Não foi possível ler a árvore AT-SPI: {error}"),
                        );
                    }
                }
                Command::Enable(false) => {
                    navigator.candidates.clear();
                    navigator.selected = None;
                    set_status(&status, "Modo itens pausado.");
                }
                Command::Move(direction) if !navigator.candidates.is_empty() => {
                    block_on(navigator.move_focus(direction, &status));
                }
                Command::Activate => block_on(navigator.activate(&status)),
                Command::Stop => break,
                Command::Move(_) => set_status(&status, "Nenhum item acessível foi encontrado."),
            }
        }
    }

    fn wait_until_stop(receiver: Receiver<Command>) {
        while !matches!(receiver.recv(), Ok(Command::Stop) | Err(_)) {}
    }

    async fn accessible<'a>(
        bus: &'a Connection,
        item: &'a ObjectRefOwned,
    ) -> Option<AccessibleProxy<'a>> {
        AccessibleProxy::builder(bus)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .destination(item.name_as_str()?)
            .ok()?
            .path(item.path())
            .ok()?
            .build()
            .await
            .ok()
    }

    async fn extents(bus: &Connection, item: &ObjectRefOwned) -> Option<Rect> {
        let component = component(bus, item).await?;
        let (x, y, width, height) = component.get_extents(CoordType::Screen).await.ok()?;
        let bounds = Rect {
            x,
            y,
            width,
            height,
        };
        bounds.valid().then_some(bounds)
    }

    async fn component<'a>(
        bus: &'a Connection,
        item: &'a ObjectRefOwned,
    ) -> Option<ComponentProxy<'a>> {
        ComponentProxy::builder(bus)
            .destination(item.name_as_str()?)
            .ok()?
            .path(item.path())
            .ok()?
            .build()
            .await
            .ok()
    }

    async fn focus(bus: &Connection, item: &ObjectRefOwned) -> bool {
        let Some(component) = component(bus, item).await else {
            return false;
        };
        let _ = component.scroll_to(ScrollType::Anywhere).await;
        component.grab_focus().await.unwrap_or(false)
    }

    async fn action<'a>(bus: &'a Connection, item: &'a ObjectRefOwned) -> Option<ActionProxy<'a>> {
        ActionProxy::builder(bus)
            .destination(item.name_as_str()?)
            .ok()?
            .path(item.path())
            .ok()?
            .build()
            .await
            .ok()
    }

    async fn activate_action(proxy: &ActionProxy<'_>) -> bool {
        let Ok(count) = proxy.nactions().await else {
            return false;
        };
        for index in 0..count {
            let name = proxy
                .get_name(index)
                .await
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ["click", "press", "activate", "open", "select", "toggle"]
                .iter()
                .any(|expected| name.contains(expected))
                && proxy.do_action(index).await.unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    fn actionable(role: Role) -> bool {
        matches!(
            role,
            Role::Button
                | Role::CheckBox
                | Role::CheckMenuItem
                | Role::ComboBox
                | Role::Entry
                | Role::Link
                | Role::ListItem
                | Role::MenuItem
                | Role::PageTab
                | Role::PasswordText
                | Role::RadioButton
                | Role::RadioMenuItem
                | Role::Slider
                | Role::SpinButton
                | Role::TableCell
                | Role::ToggleButton
                | Role::TreeItem
                | Role::PushButtonMenu
                | Role::Unknown
        )
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use ::windows::{
        Win32::{
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize,
            },
            UI::Accessibility::{
                CUIAutomation8, ExpandCollapseState_Expanded, IUIAutomation2, IUIAutomationElement,
                IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
                IUIAutomationScrollItemPattern, IUIAutomationSelectionItemPattern,
                IUIAutomationTogglePattern, IUIAutomationTreeWalker, UIA_ButtonControlTypeId,
                UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_DataItemControlTypeId,
                UIA_EditControlTypeId, UIA_ExpandCollapsePatternId, UIA_HyperlinkControlTypeId,
                UIA_InvokePatternId, UIA_ListItemControlTypeId, UIA_MenuItemControlTypeId,
                UIA_RadioButtonControlTypeId, UIA_ScrollBarControlTypeId, UIA_ScrollItemPatternId,
                UIA_SelectionItemPatternId, UIA_SliderControlTypeId, UIA_SpinnerControlTypeId,
                UIA_SplitButtonControlTypeId, UIA_TabItemControlTypeId, UIA_TogglePatternId,
                UIA_TreeItemControlTypeId, UIA_WindowControlTypeId,
            },
        },
        core::BOOL,
    };

    pub fn run(receiver: Receiver<Command>, status: Arc<Mutex<String>>) {
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                set_status(
                    &status,
                    "UI Automation indisponível: não foi possível inicializar COM.",
                );
                wait_until_stop(receiver);
                return;
            }
            match CoCreateInstance::<_, IUIAutomation2>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
            {
                Ok(automation) => {
                    let _ = automation.SetConnectionTimeout(300);
                    let _ = automation.SetTransactionTimeout(300);
                    let mut navigator = Navigator {
                        automation,
                        candidates: Vec::new(),
                        selected: None,
                    };
                    while let Ok(command) = receiver.recv() {
                        match command {
                            Command::Enable(true) | Command::Refresh => navigator.refresh(&status),
                            Command::Enable(false) => {
                                navigator.candidates.clear();
                                navigator.selected = None;
                                set_status(&status, "Modo itens pausado.");
                            }
                            Command::Move(direction) => navigator.move_focus(direction, &status),
                            Command::Activate => navigator.activate(&status),
                            Command::Stop => break,
                        }
                    }
                }
                Err(error) => set_status(&status, format!("UI Automation indisponível: {error}")),
            }
            CoUninitialize();
        }
    }

    struct Candidate {
        element: IUIAutomationElement,
        bounds: Rect,
        name: String,
        focused: bool,
    }

    struct Navigator {
        automation: IUIAutomation2,
        candidates: Vec<Candidate>,
        selected: Option<usize>,
    }

    impl Navigator {
        fn refresh(&mut self, status: &Mutex<String>) {
            set_status(status, "Lendo os itens acessíveis da janela ativa…");
            let result = unsafe { self.snapshot() };
            match result {
                Ok(candidates) => {
                    self.selected = candidates.iter().position(|candidate| candidate.focused);
                    if self.selected.is_none() {
                        self.selected = nearest_neighbor(
                            &candidates
                                .iter()
                                .map(|candidate| candidate.bounds)
                                .collect::<Vec<_>>(),
                            None,
                            Direction::Right,
                        );
                        if let Some(index) = self.selected {
                            let _ = unsafe { focus(&candidates[index].element) };
                        }
                    }
                    self.candidates = candidates;
                    if let Some(index) = self.selected {
                        set_status(status, item_label(&self.candidates[index].name));
                    } else {
                        set_status(
                            status,
                            "Esta janela não expõe itens interativos visíveis pela acessibilidade.",
                        );
                    }
                }
                Err(error) => set_status(
                    status,
                    format!("UI Automation não encontrou uma janela acessível: {error}"),
                ),
            }
        }

        unsafe fn snapshot(&self) -> ::windows::core::Result<Vec<Candidate>> {
            unsafe {
                let focused = self.automation.GetFocusedElement()?;
                let walker = self.automation.ControlViewWalker()?;
                let root = window_root(&walker, &focused)?;
                let mut pending = children(&walker, &root)?;
                let mut candidates = Vec::new();
                let mut scanned = 0;
                while let Some(element) = pending.pop() {
                    scanned += 1;
                    if candidates.len() >= MAX_ITEMS || scanned > MAX_ITEMS * 4 {
                        break;
                    }
                    if let Some(candidate) = candidate(&element) {
                        candidates.push(candidate);
                    }
                    pending.extend(children(&walker, &element).unwrap_or_default());
                }
                Ok(candidates)
            }
        }

        fn move_focus(&mut self, direction: Direction, status: &Mutex<String>) {
            let bounds = self
                .candidates
                .iter()
                .map(|candidate| candidate.bounds)
                .collect::<Vec<_>>();
            let Some(index) = nearest_neighbor(&bounds, self.selected, direction) else {
                set_status(status, "Não há outro item nessa direção.");
                return;
            };
            if unsafe { focus(&self.candidates[index].element) }.is_ok() {
                self.selected = Some(index);
                set_status(status, item_label(&self.candidates[index].name));
            } else {
                self.refresh(status);
            }
        }

        fn activate(&mut self, status: &Mutex<String>) {
            let Some(index) = self.selected else {
                set_status(status, "Nenhum item está focado para ativar.");
                return;
            };
            let element = &self.candidates[index].element;
            let result = unsafe { activate(element) };
            if result.is_ok() {
                set_status(status, "Item ativado.");
                self.refresh(status);
            } else {
                set_status(
                    status,
                    "O item focado não oferece uma ação de ativação UI Automation.",
                );
            }
        }
    }

    unsafe fn window_root(
        walker: &IUIAutomationTreeWalker,
        element: &IUIAutomationElement,
    ) -> ::windows::core::Result<IUIAutomationElement> {
        unsafe {
            let mut root = element.clone();
            for _ in 0..32 {
                if root.CurrentControlType()? == UIA_WindowControlTypeId {
                    return Ok(root);
                }
                root = walker.GetParentElement(&root)?;
            }
            if root.CurrentControlType()? == UIA_WindowControlTypeId {
                Ok(root)
            } else {
                Err(::windows::core::Error::new(
                    ::windows::core::HRESULT(0x80004005u32 as i32),
                    "Focused element is not inside a UI Automation window",
                ))
            }
        }
    }

    unsafe fn children(
        walker: &IUIAutomationTreeWalker,
        element: &IUIAutomationElement,
    ) -> ::windows::core::Result<Vec<IUIAutomationElement>> {
        unsafe {
            let mut result = Vec::new();
            let Ok(mut child) = walker.GetFirstChildElement(element) else {
                return Ok(result);
            };
            loop {
                result.push(child.clone());
                if result.len() >= MAX_ITEMS {
                    break;
                }
                let Ok(next) = walker.GetNextSiblingElement(&child) else {
                    break;
                };
                child = next;
            }
            Ok(result)
        }
    }

    fn candidate(element: &IUIAutomationElement) -> Option<Candidate> {
        unsafe {
            let control_type = element.CurrentControlType().ok()?;
            let focusable = element.CurrentIsKeyboardFocusable().ok()?.as_bool();
            let enabled = element.CurrentIsEnabled().ok()?.as_bool();
            let offscreen = element.CurrentIsOffscreen().ok()?.as_bool();
            let rect = element.CurrentBoundingRectangle().ok()?;
            let bounds = Rect {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            };
            if !focusable || !enabled || offscreen || !bounds.valid() || !interactive(control_type)
            {
                return None;
            }
            Some(Candidate {
                element: element.clone(),
                bounds,
                name: element
                    .CurrentName()
                    .map(|name| name.to_string())
                    .unwrap_or_default(),
                focused: element.CurrentHasKeyboardFocus().is_ok_and(BOOL::as_bool),
            })
        }
    }

    fn interactive(control_type: ::windows::Win32::UI::Accessibility::UIA_CONTROLTYPE_ID) -> bool {
        [
            UIA_ButtonControlTypeId,
            UIA_CheckBoxControlTypeId,
            UIA_ComboBoxControlTypeId,
            UIA_DataItemControlTypeId,
            UIA_EditControlTypeId,
            UIA_HyperlinkControlTypeId,
            UIA_ListItemControlTypeId,
            UIA_MenuItemControlTypeId,
            UIA_RadioButtonControlTypeId,
            UIA_ScrollBarControlTypeId,
            UIA_SliderControlTypeId,
            UIA_SpinnerControlTypeId,
            UIA_SplitButtonControlTypeId,
            UIA_TabItemControlTypeId,
            UIA_TreeItemControlTypeId,
        ]
        .contains(&control_type)
    }

    unsafe fn focus(element: &IUIAutomationElement) -> ::windows::core::Result<()> {
        unsafe {
            if let Ok(scroll) = element
                .GetCurrentPatternAs::<IUIAutomationScrollItemPattern>(UIA_ScrollItemPatternId)
            {
                let _ = scroll.ScrollIntoView();
            }
            element.SetFocus()
        }
    }

    unsafe fn activate(element: &IUIAutomationElement) -> ::windows::core::Result<()> {
        unsafe {
            if let Ok(invoke) =
                element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
            {
                return invoke.Invoke();
            }
            if let Ok(select) = element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                UIA_SelectionItemPatternId,
            ) {
                return select.Select();
            }
            if let Ok(toggle) =
                element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
            {
                return toggle.Toggle();
            }
            if let Ok(expand) = element.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                UIA_ExpandCollapsePatternId,
            ) {
                return if expand.CurrentExpandCollapseState()? == ExpandCollapseState_Expanded {
                    expand.Collapse()
                } else {
                    expand.Expand()
                };
            }
            Err(::windows::core::Error::new(
                ::windows::core::HRESULT(0x80004001u32 as i32),
                "Item does not expose an activation pattern",
            ))
        }
    }

    fn wait_until_stop(receiver: Receiver<Command>) {
        while !matches!(receiver.recv(), Ok(Command::Stop) | Err(_)) {}
    }
}

fn item_label(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        "Item acessível sem nome".to_owned()
    } else {
        format!("Foco: {}", name.chars().take(100).collect::<String>())
    }
}
