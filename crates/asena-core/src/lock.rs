//! Lock readiness and authentication are independent trust boundaries.
#[derive(Debug, Default)]
pub struct LockState {
    ready: bool,
    epoch: u64,
    worker: Option<u64>,
    cancelled: bool,
    authenticated: bool,
}
impl LockState {
    pub fn ready(&self) -> bool {
        self.ready
    }
    pub fn compositor_locked(&mut self) {
        self.ready = true;
    }
    pub fn begin(&mut self) -> Option<u64> {
        if !self.ready || self.worker.is_some() || self.authenticated {
            return None;
        }
        self.epoch = self.epoch.checked_add(1)?;
        self.worker = Some(self.epoch);
        self.cancelled = false;
        Some(self.epoch)
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.authenticated = false;
    }
    pub fn complete(&mut self, ticket: u64, success: bool) -> bool {
        if self.worker != Some(ticket) {
            return false;
        }
        self.worker = None;
        self.authenticated = success && !self.cancelled && self.ready;
        self.authenticated
    }
    pub fn busy(&self) -> bool {
        self.worker.is_some()
    }
    pub fn may_unlock(&self) -> bool {
        self.authenticated && self.ready
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn readiness_and_valid_authentication_are_both_required() {
        let mut state = LockState::default();
        assert!(state.begin().is_none());
        state.compositor_locked();
        let ticket = state.begin().expect("ready");
        assert!(state.begin().is_none());
        assert!(!state.complete(ticket + 1, true));
        assert!(state.busy());
        assert!(!state.complete(ticket, false));
        assert!(!state.may_unlock());
        let ticket = state.begin().expect("retry");
        assert!(state.complete(ticket, true));
        assert!(state.may_unlock());
    }
    #[test]
    fn cancelled_late_success_cannot_unlock_or_overlap_workers() {
        let mut state = LockState::default();
        state.compositor_locked();
        let old = state.begin().expect("ready");
        state.cancel();
        assert!(state.begin().is_none());
        assert!(!state.complete(old, true));
        let new = state.begin().expect("finished old worker");
        assert!(!state.complete(old, true));
        assert!(state.complete(new, true));
    }
}
