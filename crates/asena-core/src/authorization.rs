//! Reject late native PAM completions after identity changes or cancellation.
#[derive(Default)]
pub struct Flow {
    epoch: u64,
    attempts: u8,
    cancelled: bool,
    finished: bool,
    running: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Completion {
    Ignore,
    Retry,
    Finished(bool),
}
impl Flow {
    pub fn start(&mut self) -> Option<u64> {
        if self.cancelled || self.finished || self.attempts >= 3 {
            return None;
        }
        self.epoch = self.epoch.wrapping_add(1);
        self.attempts += 1;
        self.running = true;
        Some(self.epoch)
    }
    pub fn current(&self, epoch: u64) -> bool {
        !self.cancelled && !self.finished && self.running && self.epoch == epoch
    }
    pub fn complete(&mut self, epoch: u64, gained: bool) -> Completion {
        if !self.current(epoch) {
            return Completion::Ignore;
        }
        self.running = false;
        if gained || self.attempts >= 3 {
            self.finished = true;
            Completion::Finished(gained)
        } else {
            Completion::Retry
        }
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.epoch = self.epoch.wrapping_add(1);
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn changed_identity_and_cancelled_requests_ignore_old_success() {
        let mut flow = Flow::default();
        let first = flow.start().expect("first");
        let next = flow.start().expect("next");
        assert_eq!(flow.complete(first, true), Completion::Ignore);
        flow.cancel();
        assert_eq!(flow.complete(next, true), Completion::Ignore);
        assert_eq!(flow.start(), None);
    }
    #[test]
    fn retries_are_bounded_and_success_is_terminal() {
        let mut flow = Flow::default();
        for _ in 0..2 {
            let epoch = flow.start().expect("attempt");
            assert_eq!(flow.complete(epoch, false), Completion::Retry);
        }
        let epoch = flow.start().expect("last");
        assert_eq!(flow.complete(epoch, false), Completion::Finished(false));
        assert_eq!(flow.start(), None);
        let mut flow = Flow::default();
        let epoch = flow.start().expect("attempt");
        assert_eq!(flow.complete(epoch, true), Completion::Finished(true));
        assert_eq!(flow.complete(epoch, true), Completion::Ignore);
    }
}
