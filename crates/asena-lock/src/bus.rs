use asena_core::i18n::gettext;
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct State {
    status: u8,
    waiters: Vec<async_channel::Sender<bool>>,
    sleep_owner: Option<String>,
    system_sleep: bool,
}
pub enum UiRequest {
    Focus,
    RefreshSleep,
}
#[derive(Clone, Default)]
pub struct Readiness(Arc<Mutex<State>>);
impl Readiness {
    pub fn held(&self) -> bool {
        self.0.lock().map_or(true, |state| {
            state.system_sleep || state.sleep_owner.is_some()
        })
    }
    pub fn owner(&self) -> Option<String> {
        self.0
            .lock()
            .ok()
            .and_then(|state| state.sleep_owner.clone())
    }
    pub fn hold(&self, owner: &str, pending: bool) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        if pending
            && (state.status == 2
                || state
                    .sleep_owner
                    .as_deref()
                    .is_some_and(|current| current != owner))
        {
            return false;
        }
        if !pending
            && state
                .sleep_owner
                .as_deref()
                .is_some_and(|current| current != owner)
        {
            return false;
        }
        state.sleep_owner = if pending {
            Some(owner.to_owned())
        } else {
            None
        };
        true
    }
    pub fn system_sleep(&self, pending: bool) {
        if let Ok(mut state) = self.0.lock() {
            state.system_sleep = pending;
        }
    }
    pub fn owner_gone(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.sleep_owner = None;
        }
    }
    pub fn begin_unlock(&self) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        if state.status != 1 || state.system_sleep || state.sleep_owner.is_some() {
            return false;
        }
        state.status = 2;
        for waiter in state.waiters.drain(..) {
            let _ = waiter.try_send(false);
        }
        true
    }
    pub fn finish(&self, ready: bool) {
        if let Ok(mut state) = self.0.lock() {
            state.status = if ready { 1 } else { 2 };
            for waiter in state.waiters.drain(..) {
                let _ = waiter.try_send(ready);
            }
        }
    }
    fn ready(&self) -> bool {
        self.0.lock().is_ok_and(|state| state.status == 1)
    }
}
pub struct LockBus {
    pub readiness: Readiness,
    pub focus: async_channel::Sender<UiRequest>,
    pub scope: String,
}
#[zbus::interface(name = "org.asenalinux.Lock")]
impl LockBus {
    async fn set_sleep_pending(
        &self,
        scope: &str,
        pending: bool,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        if self.scope != scope {
            return Err(zbus::fdo::Error::AccessDenied(gettext(
                "Run this command in the session you want to lock.",
            )));
        }
        let owner = header.sender().ok_or_else(|| {
            zbus::fdo::Error::AccessDenied(gettext("The sleep request has no owner."))
        })?;
        if !self.readiness.hold(owner.as_str(), pending) {
            return Err(zbus::fdo::Error::Failed(gettext(
                "The lock state changed while preparing to suspend.",
            )));
        }
        self.focus
            .send(UiRequest::RefreshSleep)
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The session could not be locked.")))
    }
    #[zbus(property)]
    fn ready(&self) -> bool {
        self.readiness.ready()
    }
    #[zbus(property)]
    fn sleep_pending(&self) -> bool {
        self.readiness.held()
    }
    async fn wait_ready(&self, scope: &str) -> zbus::fdo::Result<()> {
        if self.scope != scope {
            return Err(zbus::fdo::Error::AccessDenied(gettext(
                "Run this command in the session you want to lock.",
            )));
        }
        let receiver = {
            let mut state = self.readiness.0.lock().map_err(|_| {
                zbus::fdo::Error::Failed(gettext("The session could not be locked."))
            })?;
            match state.status {
                1 => return Ok(()),
                2 => {
                    return Err(zbus::fdo::Error::Failed(gettext(
                        "The session could not be locked.",
                    )))
                }
                _ => {}
            }
            state.waiters.retain(|waiter| !waiter.is_closed());
            if state.waiters.len() >= 16 {
                return Err(zbus::fdo::Error::LimitsExceeded(gettext(
                    "Too many lock requests.",
                )));
            }
            let (sender, receiver) = async_channel::bounded(1);
            state.waiters.push(sender);
            receiver
        };
        if receiver.recv().await.unwrap_or(false) && self.readiness.ready() {
            Ok(())
        } else {
            Err(zbus::fdo::Error::Failed(gettext(
                "The session could not be locked.",
            )))
        }
    }
    fn focus(&self) {
        let _ = self.focus.try_send(UiRequest::Focus);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_sleep_hold_and_unlock_transition_are_mutually_exclusive() {
        let ready = Readiness::default();
        ready.finish(true);
        assert!(ready.hold(":1.10", true));
        assert!(!ready.begin_unlock());
        assert!(!ready.hold(":1.11", false));
        assert!(ready.held());
        assert!(ready.hold(":1.10", false));
        assert!(ready.begin_unlock());
        assert!(!ready.ready());
        assert!(!ready.hold(":1.10", true));
    }
    #[test]
    fn resume_clears_holds_without_unlocking_the_session() {
        let ready = Readiness::default();
        ready.finish(true);
        assert!(ready.hold(":1.10", true));
        ready.system_sleep(true);
        ready.owner_gone();
        assert!(ready.held());
        ready.system_sleep(false);
        assert!(!ready.held());
        assert!(ready.ready());
    }
    #[test]
    fn resume_preserves_a_live_screen_power_owner_until_restore() {
        let ready = Readiness::default();
        ready.finish(true);
        assert!(ready.hold(":1.10", true));
        ready.system_sleep(true);
        ready.system_sleep(false);
        assert!(ready.held());
        assert!(!ready.begin_unlock());
        assert!(ready.hold(":1.10", false));
        assert!(ready.begin_unlock());
    }
}
