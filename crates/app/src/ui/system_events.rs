//! Blocking native notifications; no periodic desktop polling.
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        randr::{ConnectionExt as _, NotifyMask},
        xproto::{
            CLIENT_MESSAGE_EVENT, ClientMessageEvent, ConnectionExt as _, CreateWindowAux,
            EventMask, WindowClass,
        },
    },
};

pub struct SystemEvents {
    control: u32,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    _resume: ResumeSignals,
}
impl SystemEvents {
    pub fn start(changed: Arc<dyn Fn() + Send + Sync>) -> io::Result<Self> {
        let (connection, screen) = x11rb::connect(None).map_err(io::Error::other)?;
        let root = connection.setup().roots[screen].root;
        let control = connection.generate_id().map_err(io::Error::other)?;
        connection
            .create_window(
                0,
                control,
                root,
                0,
                0,
                1,
                1,
                0,
                WindowClass::INPUT_ONLY,
                0,
                &CreateWindowAux::new().event_mask(EventMask::STRUCTURE_NOTIFY),
            )
            .map_err(io::Error::other)?
            .check()
            .map_err(io::Error::other)?;
        connection
            .randr_select_input(
                root,
                NotifyMask::SCREEN_CHANGE
                    | NotifyMask::CRTC_CHANGE
                    | NotifyMask::OUTPUT_CHANGE
                    | NotifyMask::RESOURCE_CHANGE,
            )
            .map_err(io::Error::other)?
            .check()
            .map_err(io::Error::other)?;
        connection
            .change_window_attributes(
                root,
                &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
                    .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
            )
            .map_err(io::Error::other)?
            .check()
            .map_err(io::Error::other)?;
        let workarea = connection
            .intern_atom(false, b"_NET_WORKAREA")
            .map_err(io::Error::other)?
            .reply()
            .map_err(io::Error::other)?
            .atom;
        connection.flush().map_err(io::Error::other)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let notify = changed.clone();
        let thread = std::thread::Builder::new()
            .name("athanor-displays".into())
            .spawn(move || {
                while let Ok(event) = connection.wait_for_event() {
                    if stopping.load(Ordering::Acquire) {
                        break;
                    }
                    let changed = match event {
                        Event::RandrScreenChangeNotify(_) | Event::RandrNotify(_) => true,
                        Event::ConfigureNotify(event) => {
                            event.window == root || event.window == control
                        }
                        Event::PropertyNotify(event) => {
                            event.window == root && event.atom == workarea
                        }
                        _ => false,
                    };
                    if changed {
                        notify();
                    }
                }
            })?;
        Ok(Self {
            control,
            stop,
            thread: Some(thread),
            _resume: ResumeSignals::start(changed),
        })
    }
    #[cfg(test)]
    fn probe_for_test(&self) {
        use x11rb::protocol::xproto::{CONFIGURE_NOTIFY_EVENT, ConfigureNotifyEvent};
        let (connection, _) = x11rb::connect(None).unwrap();
        let event = ConfigureNotifyEvent {
            response_type: CONFIGURE_NOTIFY_EVENT,
            sequence: 0,
            event: self.control,
            window: self.control,
            above_sibling: 0,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            border_width: 0,
            override_redirect: false,
        };
        connection
            .send_event(false, self.control, EventMask::NO_EVENT, event)
            .unwrap()
            .check()
            .unwrap();
        connection.flush().unwrap();
    }
}
impl Drop for SystemEvents {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok((connection, _)) = x11rb::connect(None) {
            let event = ClientMessageEvent {
                response_type: CLIENT_MESSAGE_EVENT,
                format: 32,
                sequence: 0,
                window: self.control,
                type_: 0,
                data: [0u32; 5].into(),
            };
            if let Ok(cookie) =
                connection.send_event(false, self.control, EventMask::NO_EVENT, event)
            {
                // Round-trip before dropping the sending connection: otherwise X11 can
                // discard the wake request and leave the receiver blocked during join.
                let _ = cookie.check();
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
struct ResumeSignals {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}
impl ResumeSignals {
    fn start(changed: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("athanor-resume".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(async move {
                    let listen = async move {
                        use futures_util::StreamExt;
                        let Ok(connection) = zbus::Connection::system().await else {
                            return;
                        };
                        let Ok(proxy) = zbus::Proxy::new(
                            &connection,
                            "org.freedesktop.login1",
                            "/org/freedesktop/login1",
                            "org.freedesktop.login1.Manager",
                        )
                        .await
                        else {
                            return;
                        };
                        let Ok(mut signals) = proxy.receive_signal("PrepareForSleep").await else {
                            return;
                        };
                        while let Some(signal) = signals.next().await {
                            if matches!(signal.body().deserialize::<bool>(), Ok(false)) {
                                changed();
                            }
                        }
                    };
                    tokio::select! {_=listen=>{},_=stopped=>{}}
                });
            })
            .ok();
        Self {
            stop: Some(stop),
            thread,
        }
    }
}
impl Drop for ResumeSignals {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a live X11 server"]
    fn a_native_display_notification_wakes_recovery_and_the_listener_shuts_down() {
        use std::{
            sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
            },
            time::{Duration, Instant},
        };
        let count = Arc::new(AtomicUsize::new(0));
        let notified = count.clone();
        let listener = SystemEvents::start(Arc::new(move || {
            notified.fetch_add(1, Ordering::Relaxed);
        }))
        .unwrap();
        listener.probe_for_test();
        let deadline = Instant::now() + Duration::from_secs(2);
        while count.load(Ordering::Relaxed) == 0 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        let stopping = Instant::now();
        drop(listener);
        assert!(
            stopping.elapsed() < Duration::from_secs(1),
            "native listener shutdown must be bounded"
        );
    }
}
