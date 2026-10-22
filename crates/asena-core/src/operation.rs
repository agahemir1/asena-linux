//! Service ownership epochs prevent stale replies from changing current state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    epoch: u64,
    serial: u64,
}
#[derive(Default)]
pub struct OperationGate {
    epoch: u64,
    serial: u64,
    pending: Option<Ticket>,
    available: bool,
}
impl OperationGate {
    pub fn owner_changed(&mut self, available: bool) {
        self.epoch = self.epoch.wrapping_add(1);
        self.pending = None;
        self.available = available;
    }
    pub fn begin(&mut self) -> Option<Ticket> {
        if !self.available || self.pending.is_some() {
            return None;
        }
        self.serial = self.serial.wrapping_add(1);
        let ticket = Ticket {
            epoch: self.epoch,
            serial: self.serial,
        };
        self.pending = Some(ticket);
        Some(ticket)
    }
    pub fn finish(&mut self, ticket: Ticket) -> bool {
        if self.pending != Some(ticket) {
            return false;
        }
        self.pending = None;
        true
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
}
pub fn stepped_level(value: f64, step: f64, minimum: f64) -> Option<f64> {
    if !value.is_finite() || !step.is_finite() || !(0.0..=1.0).contains(&minimum) {
        return None;
    }
    Some((value + step).clamp(minimum, 1.0))
}
pub fn brightness_raw(level: f64, maximum: u32) -> Option<u32> {
    if maximum == 0 || !level.is_finite() {
        return None;
    }
    // Keep at least 5% of the hardware range lit, with a minimum of one step.
    Some(
        (level.clamp(0.05, 1.0) * f64::from(maximum))
            .round()
            .max(1.0) as u32,
    )
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn disconnect_and_reconnect_reject_old_replies() {
        let mut gate = OperationGate::default();
        assert!(gate.begin().is_none());
        gate.owner_changed(true);
        let first = gate.begin().expect("ready");
        assert!(gate.begin().is_none());
        gate.owner_changed(false);
        assert!(!gate.finish(first));
        gate.owner_changed(true);
        let second = gate.begin().expect("reconnected");
        assert!(!gate.finish(first));
        assert!(gate.busy());
        assert!(gate.finish(second));
        assert!(!gate.busy());
        assert!(!gate.finish(second));
    }
    #[test]
    fn levels_are_bounded_and_invalid_numbers_rejected() {
        assert_eq!(stepped_level(0.98, 0.05, 0.0), Some(1.0));
        assert_eq!(stepped_level(0.01, -0.1, 0.05), Some(0.05));
        assert_eq!(stepped_level(f64::NAN, 0.1, 0.0), None);
        assert_eq!(brightness_raw(0.0, 100), Some(5));
        assert_eq!(brightness_raw(0.0, 1), Some(1));
        assert_eq!(brightness_raw(0.5, 0), None);
    }
}
