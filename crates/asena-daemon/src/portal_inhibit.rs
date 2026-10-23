//! xdg-desktop-portal Inhibit and Background backends (K22). gtk's Inhibit
//! backend needs GNOME's session manager, and no backend for Background ships
//! with Debian, so sandboxed applications could neither keep the screen awake
//! nor ask to run in the background.
//!
//! An idle inhibition pauses Asena's idle actions (dim, lock, screen off,
//! suspend); a suspend inhibition holds a logind "sleep" inhibitor. Both last
//! until the portal closes the request object at `handle`.
use crate::power;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use zbus::{
    object_server::ObjectServer,
    zvariant::{OwnedFd, OwnedObjectPath, OwnedValue},
};

/// Portal inhibit flags.
const SUSPEND: u32 = 4;
const IDLE: u32 = 8;

#[derive(Default)]
struct Inhibitors {
    idle: HashSet<OwnedObjectPath>,
    /// Dropping the descriptor releases logind's inhibitor.
    sleep: HashMap<OwnedObjectPath, OwnedFd>,
}

/// Idle actions stay paused while any idle inhibition is held.
fn changed_idle(before: usize, after: usize) -> Option<bool> {
    match (before, after) {
        (0, 1..) => Some(true),
        (1.., 0) => Some(false),
        _ => None,
    }
}

#[derive(Clone)]
pub struct Inhibit {
    state: Arc<Mutex<Inhibitors>>,
    power: async_channel::Sender<power::Command>,
}

impl Inhibit {
    pub fn new(power: async_channel::Sender<power::Command>) -> Self {
        Self {
            state: Arc::default(),
            power,
        }
    }

    /// Runs `watch` for this backend's inhibitions.
    pub fn watcher(&self, connection: zbus::Connection) -> impl std::future::Future<Output = ()> {
        watch(connection, self.state.clone(), self.power.clone())
    }
}

async fn sleep_inhibitor(who: &str, why: &str) -> zbus::Result<OwnedFd> {
    let system = zbus::Connection::system().await?;
    let manager = zbus::Proxy::new(
        &system,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;
    manager.call("Inhibit", &("sleep", who, why, "block")).await
}

#[zbus::interface(name = "org.freedesktop.impl.portal.Inhibit")]
impl Inhibit {
    // The portal specification fixes the arguments; zbus adds the injected ones.
    #[allow(clippy::too_many_arguments)]
    async fn inhibit(
        &self,
        handle: OwnedObjectPath,
        app_id: &str,
        _window: &str,
        flags: u32,
        options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &ObjectServer,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        let reason = options
            .get("reason")
            .and_then(|value| String::try_from(value.clone()).ok())
            .unwrap_or_default();
        let who = if app_id.is_empty() { "Asena" } else { app_id };
        let sleep = if flags & SUSPEND != 0 {
            match sleep_inhibitor(who, &reason).await {
                Ok(fd) => Some(fd),
                Err(error) => {
                    tracing::warn!(%error, "Could not inhibit sleep");
                    None
                }
            }
        } else {
            None
        };
        let pause = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| zbus::fdo::Error::Failed("inhibitor state".into()))?;
            let before = state.idle.len();
            if flags & IDLE != 0 {
                state.idle.insert(handle.clone());
            }
            if let Some(fd) = sleep {
                state.sleep.insert(handle.clone(), fd);
            }
            changed_idle(before, state.idle.len())
        };
        if let Some(paused) = pause {
            let _ = self.power.send(power::Command::Inhibit(paused)).await;
        }
        server
            .at(
                handle.clone(),
                Request {
                    handle: handle.clone(),
                    state: self.state.clone(),
                    power: self.power.clone(),
                },
            )
            .await?;
        // A caller that already left never closes its request: the portal only
        // watches callers whose request it had exported before they left.
        if let Some(sender) = sender_of(handle.as_str()) {
            let present = match (
                zbus::fdo::DBusProxy::new(connection).await,
                zbus::names::BusName::try_from(sender.as_str()),
            ) {
                (Ok(bus), Ok(name)) => bus.name_has_owner(name).await.unwrap_or(true),
                _ => true,
            };
            if !present {
                release(&self.state, &self.power, server, &handle).await;
            }
        }
        Ok(())
    }

    /// Session state monitoring needs a session manager, which Asena does not have.
    async fn create_monitor(
        &self,
        _handle: OwnedObjectPath,
        _session_handle: OwnedObjectPath,
        _app_id: &str,
        _window: &str,
    ) -> u32 {
        2
    }

    async fn query_end_response(&self, _session_handle: OwnedObjectPath) {}
}

struct Request {
    handle: OwnedObjectPath,
    state: Arc<Mutex<Inhibitors>>,
    power: async_channel::Sender<power::Command>,
}

#[zbus::interface(name = "org.freedesktop.impl.portal.Request")]
impl Request {
    async fn close(&self, #[zbus(object_server)] server: &ObjectServer) {
        release(&self.state, &self.power, server, &self.handle).await;
    }
}

/// The caller's unique name, which the portal encodes in the request handle
/// (`…/request/1_31/token` for `:1.31`).
fn sender_of(handle: &str) -> Option<String> {
    let token = handle
        .strip_prefix("/org/freedesktop/portal/desktop/request/")?
        .split('/')
        .next()?;
    (!token.is_empty()).then(|| format!(":{}", token.replace('_', ".")))
}

/// Ends one inhibition and removes its request object.
async fn release(
    state: &Arc<Mutex<Inhibitors>>,
    power: &async_channel::Sender<power::Command>,
    server: &ObjectServer,
    handle: &OwnedObjectPath,
) {
    let resume = match state.lock() {
        Ok(mut state) => {
            let before = state.idle.len();
            state.idle.remove(handle);
            state.sleep.remove(handle);
            changed_idle(before, state.idle.len())
        }
        Err(_) => None,
    };
    if let Some(paused) = resume {
        let _ = power.send(power::Command::Inhibit(paused)).await;
    }
    let _ = server.remove::<Request, _>(handle).await;
}

/// Ends the inhibitions of callers that leave the bus without closing them.
async fn watch(
    connection: zbus::Connection,
    inhibit: Arc<Mutex<Inhibitors>>,
    power: async_channel::Sender<power::Command>,
) {
    use futures_lite::StreamExt;
    let Ok(bus) = zbus::fdo::DBusProxy::new(&connection).await else {
        return;
    };
    let Ok(mut changes) = bus.receive_name_owner_changed().await else {
        return;
    };
    while let Some(change) = changes.next().await {
        let Ok(args) = change.args() else {
            continue;
        };
        if args.new_owner().is_some() {
            continue;
        }
        let name = args.name().to_string();
        let handles: Vec<OwnedObjectPath> = match inhibit.lock() {
            Ok(state) => state
                .idle
                .iter()
                .chain(state.sleep.keys())
                .filter(|handle| sender_of(handle.as_str()).as_deref() == Some(name.as_str()))
                .cloned()
                .collect(),
            Err(_) => continue,
        };
        for handle in handles {
            release(&inhibit, &power, connection.object_server(), &handle).await;
        }
    }
}

/// xdg-desktop-portal 1.20 writes autostart files itself; the backend only
/// reports which applications run without a window, and Asena does not track
/// that yet, so none are reported and background requests are allowed.
pub struct Background;

#[zbus::interface(name = "org.freedesktop.impl.portal.Background")]
impl Background {
    async fn get_app_state(&self) -> HashMap<String, OwnedValue> {
        HashMap::new()
    }

    async fn notify_background(
        &self,
        _handle: OwnedObjectPath,
        _app_id: &str,
        _name: &str,
    ) -> (u32, HashMap<String, OwnedValue>) {
        // 0 is success; result 1 allows the application to keep running.
        (
            0,
            HashMap::from([("result".to_owned(), OwnedValue::from(1u32))]),
        )
    }

    async fn enable_autostart(
        &self,
        _app_id: &str,
        _enable: bool,
        _commandline: Vec<String>,
        _flags: u32,
    ) -> bool {
        false
    }

    #[zbus(signal)]
    async fn running_applications_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_actions_pause_with_the_first_inhibition_and_resume_with_the_last() {
        assert_eq!(changed_idle(0, 1), Some(true));
        assert_eq!(changed_idle(1, 2), None);
        assert_eq!(changed_idle(2, 1), None);
        assert_eq!(changed_idle(1, 0), Some(false));
        assert_eq!(changed_idle(0, 0), None);
    }

    #[test]
    fn the_caller_comes_from_the_request_handle() {
        assert_eq!(
            sender_of("/org/freedesktop/portal/desktop/request/1_31/t").as_deref(),
            Some(":1.31")
        );
        assert_eq!(
            sender_of("/org/freedesktop/portal/desktop/request/1_204/gtk3").as_deref(),
            Some(":1.204")
        );
        assert_eq!(
            sender_of("/org/freedesktop/portal/desktop/session/1_31/t"),
            None
        );
    }
}
