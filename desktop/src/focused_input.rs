//! Geometry only: never reads text, changes focus, or runs on the gamepad thread.
use crate::placement::Rect;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusedInput {
    pub bounds: Rect,
    /// UI Automation coordinates and this work area are physical pixels.
    /// AT-SPI coordinates are compositor screen coordinates.
    pub screen: Option<Rect>,
}

pub struct InputTracker {
    latest: Arc<Mutex<Option<FocusedInput>>>,
    stop: Arc<AtomicBool>,
}

impl Default for InputTracker {
    fn default() -> Self {
        let latest = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (value, stopping) = (Arc::clone(&latest), Arc::clone(&stop));
        std::thread::spawn(move || {
            #[cfg(target_os = "linux")]
            if let Err(error) = futures_lite::future::block_on(linux::track(&value, &stopping)) {
                eprintln!("Posição automática do teclado indisponível (AT-SPI): {error}");
            }
            #[cfg(target_os = "windows")]
            windows_tracker::track(&value, &stopping);
            *value.lock().expect("focused input geometry") = None;
        });
        Self { latest, stop }
    }
}

impl InputTracker {
    pub fn latest(&self) -> Option<FocusedInput> {
        *self.latest.lock().expect("focused input geometry")
    }
}

impl Drop for InputTracker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use atspi::{
        CoordType, ObjectRefOwned, State,
        events::{focus::FocusEvent, object::StateChangedEvent},
        proxy::{
            accessible::AccessibleProxy, bus::BusProxy, component::ComponentProxy,
            registry::RegistryProxy,
        },
    };
    use futures_lite::{StreamExt, future::race};
    use zbus::{Connection, MessageStream, connection::Builder};

    async fn accessible<'a>(
        bus: &'a Connection,
        item: &'a ObjectRefOwned,
    ) -> zbus::Result<AccessibleProxy<'a>> {
        AccessibleProxy::builder(bus)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .destination(
                item.name_as_str()
                    .ok_or(zbus::Error::Failure("null accessible".into()))?,
            )?
            .path(item.path())?
            .build()
            .await
    }

    async fn geometry(bus: &Connection, item: &ObjectRefOwned) -> Option<FocusedInput> {
        let proxy = accessible(bus, item).await.ok()?;
        let state = proxy.get_state().await.ok()?;
        if ![
            State::Focused,
            State::Editable,
            State::Showing,
            State::Visible,
        ]
        .into_iter()
        .all(|s| state.contains(s))
        {
            return None;
        }
        let component = ComponentProxy::builder(bus)
            .destination(item.name_as_str()?)
            .ok()?
            .path(item.path())
            .ok()?
            .build()
            .await
            .ok()?;
        let (x, y, width, height) = component.get_extents(CoordType::Screen).await.ok()?;
        // Some Wayland clients label window-relative coordinates as screen
        // coordinates. Reject that ambiguous case rather than cover the field.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            let window = component.get_extents(CoordType::Window).await.ok()?;
            if (x, y) == (window.0, window.1) {
                return None;
            }
        }
        let bounds = Rect {
            x,
            y,
            width,
            height,
        };
        bounds.valid().then_some(FocusedInput {
            bounds,
            screen: None,
        })
    }

    pub async fn track(
        latest: &Mutex<Option<FocusedInput>>,
        stop: &AtomicBool,
    ) -> zbus::Result<()> {
        let session = Builder::session()?
            .method_timeout(Duration::from_millis(500))
            .build()
            .await?;
        let address = BusProxy::new(&session).await?.get_address().await?;
        let bus = Builder::address(address.as_str())?
            .method_timeout(Duration::from_millis(300))
            .build()
            .await?;
        let object_rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.a11y.atspi.Event.Object")?
            .member("StateChanged")?
            .build();
        let focus_rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.a11y.atspi.Event.Focus")?
            .build();
        let mut stream = futures_lite::stream::or(
            MessageStream::for_match_rule(object_rule, &bus, Some(64)).await?,
            MessageStream::for_match_rule(focus_rule, &bus, Some(64)).await?,
        );
        let registry = RegistryProxy::new(&bus).await?;
        registry
            .register_event("object:state-changed:focused")
            .await?;
        registry.register_event("focus:").await?;

        // A bounded initial search also handles a field focused before startup.
        let root = AccessibleProxy::builder(&bus)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .destination("org.a11y.atspi.Registry")?
            .path("/org/a11y/atspi/accessible/root")?
            .build()
            .await?;
        let mut pending = root.get_children().await?;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut focused = None;
        for _ in 0..1000 {
            if stop.load(Ordering::Relaxed) || std::time::Instant::now() > deadline {
                break;
            }
            let Some(item) = pending.pop() else {
                break;
            };
            if let Some(bounds) = geometry(&bus, &item).await {
                *latest.lock().expect("focused input geometry") = Some(bounds);
                focused = Some(item);
                break;
            }
            if let Ok(proxy) = accessible(&bus, &item).await
                && let Ok(children) = proxy.get_children().await
            {
                pending.extend(children);
            }
        }
        while !stop.load(Ordering::Relaxed) {
            let message = race(async { Some(stream.next().await) }, async {
                async_io::Timer::after(Duration::from_millis(250)).await;
                None
            })
            .await;
            if let Some(message) = message {
                let message =
                    message.ok_or_else(|| zbus::Error::Failure("AT-SPI desconectado".into()))??;
                if let Ok(event) = StateChangedEvent::try_from(&message)
                    && event.state == State::Focused
                {
                    if event.enabled {
                        focused = Some(event.item);
                    } else if focused.as_ref() == Some(&event.item) {
                        focused = None;
                    }
                } else if let Ok(event) = FocusEvent::try_from(&message) {
                    focused = Some(event.item);
                } else {
                    continue;
                }
            }
            let bounds = match &focused {
                Some(item) => geometry(&bus, item).await,
                None => None,
            };
            *latest.lock().expect("focused input geometry") = bounds;
        }
        registry
            .deregister_event("object:state-changed:focused")
            .await?;
        registry.deregister_event("focus:").await?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        struct Accessible(Arc<Mutex<atspi::StateSet>>);
        #[zbus::interface(name = "org.a11y.atspi.Accessible")]
        impl Accessible {
            fn get_state(&self) -> atspi::StateSet {
                *self.0.lock().unwrap()
            }
        }
        struct Component(Arc<Mutex<(i32, i32, i32, i32)>>);
        #[zbus::interface(name = "org.a11y.atspi.Component")]
        impl Component {
            fn get_extents(&self, coord_type: u32) -> (i32, i32, i32, i32) {
                let (x, y, w, h) = *self.0.lock().unwrap();
                if coord_type == 0 {
                    (x, y, w, h)
                } else {
                    (20, 30, w, h)
                }
            }
        }

        #[test]
        #[ignore = "run under dbus-run-session; uses only a synthetic accessibility provider"]
        fn atspi_tracks_bounds_and_rejects_hidden_or_unfocused_fields() {
            futures_lite::future::block_on(async {
                let states = Arc::new(Mutex::new(
                    [
                        State::Focused,
                        State::Editable,
                        State::Showing,
                        State::Visible,
                    ]
                    .into_iter()
                    .collect::<atspi::StateSet>(),
                ));
                let bounds = Arc::new(Mutex::new((100, 600, 300, 40)));
                let path = "/org/a11y/atspi/accessible/test";
                let server = Builder::session()
                    .unwrap()
                    .serve_at(path, Accessible(Arc::clone(&states)))
                    .unwrap()
                    .serve_at(path, Component(Arc::clone(&bounds)))
                    .unwrap()
                    .build()
                    .await
                    .unwrap();
                let client = Builder::session()
                    .unwrap()
                    .method_timeout(Duration::from_millis(500))
                    .build()
                    .await
                    .unwrap();
                let item = atspi::ObjectRef::new_owned(
                    server.unique_name().unwrap().clone(),
                    zbus::zvariant::ObjectPath::try_from(path).unwrap(),
                );
                let first = geometry(&client, &item).await.unwrap();
                assert_eq!(first.bounds.y, 600);
                *bounds.lock().unwrap() = (200, 700, 400, 50);
                assert_eq!(geometry(&client, &item).await.unwrap().bounds.y, 700);
                for absent in [
                    State::Focused,
                    State::Editable,
                    State::Showing,
                    State::Visible,
                ] {
                    *states.lock().unwrap() = [
                        State::Focused,
                        State::Editable,
                        State::Showing,
                        State::Visible,
                    ]
                    .into_iter()
                    .filter(|s| *s != absent)
                    .collect();
                    assert_eq!(geometry(&client, &item).await, None);
                }
            });
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_tracker {
    use super::*;
    use windows::Win32::{
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect},
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
        UI::Accessibility::{
            CUIAutomation8, IUIAutomation2, IUIAutomationTextEditPattern, UIA_EditControlTypeId,
            UIA_TextEditPatternId,
        },
    };

    fn focused(automation: &IUIAutomation2) -> windows::core::Result<Option<FocusedInput>> {
        // COM and UIA objects stay on this initialized worker thread.
        unsafe {
            let element = automation.GetFocusedElement()?;
            if element.CurrentControlType()? != UIA_EditControlTypeId
                && element
                    .GetCurrentPatternAs::<IUIAutomationTextEditPattern>(UIA_TextEditPatternId)
                    .is_err()
            {
                return Ok(None);
            }
            let r = element.CurrentBoundingRectangle()?;
            let bounds = Rect {
                x: r.left,
                y: r.top,
                width: r.right - r.left,
                height: r.bottom - r.top,
            };
            if !bounds.valid() || element.CurrentIsOffscreen()?.as_bool() {
                return Ok(None);
            }
            let mut monitor = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let screen =
                GetMonitorInfoW(MonitorFromRect(&r, MONITOR_DEFAULTTONEAREST), &mut monitor)
                    .as_bool()
                    .then(|| {
                        let r = monitor.rcWork;
                        Rect {
                            x: r.left,
                            y: r.top,
                            width: r.right - r.left,
                            height: r.bottom - r.top,
                        }
                    });
            Ok(Some(FocusedInput { bounds, screen }))
        }
    }

    pub fn track(latest: &Mutex<Option<FocusedInput>>, stop: &AtomicBool) {
        unsafe {
            use windows::Win32::UI::HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
            };
            let previous_dpi =
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return;
            }
            if let Ok(automation) =
                CoCreateInstance::<_, IUIAutomation2>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
            {
                let _ = automation.SetConnectionTimeout(300);
                let _ = automation.SetTransactionTimeout(300);
                while !stop.load(Ordering::Relaxed) {
                    *latest.lock().expect("focused input geometry") =
                        focused(&automation).ok().flatten();
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
            CoUninitialize();
            SetThreadDpiAwarenessContext(previous_dpi);
        }
    }
}
