//! Validated idle ordering and conditional brightness restoration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeouts {
    pub dim: u32,
    pub lock: u32,
    pub off: u32,
    pub suspend: u32,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvalidTimeouts {
    #[error("An idle timeout exceeds one day")]
    TooLong,
    #[error("Screen off and suspend require a lock timeout")]
    LockRequired,
    #[error("Enabled idle timeouts must be strictly increasing")]
    Order,
}
impl Timeouts {
    /// Zero disables a step. Screen off and sleep may never precede locking.
    pub fn validate(self) -> Result<Self, InvalidTimeouts> {
        let values = [self.dim, self.lock, self.off, self.suspend];
        if values.iter().any(|value| *value > 86_400) {
            return Err(InvalidTimeouts::TooLong);
        }
        if self.lock == 0 && (self.off > 0 || self.suspend > 0) {
            return Err(InvalidTimeouts::LockRequired);
        }
        let mut previous = 0;
        for value in values.into_iter().filter(|value| *value > 0) {
            if value <= previous {
                return Err(InvalidTimeouts::Order);
            }
            previous = value;
        }
        Ok(self)
    }
}
/// Restore only the exact dimmed state we own; respect intervening user changes.
#[derive(Clone, Copy, Debug)]
pub struct DimLease {
    original: f64,
    applied: f64,
}
impl DimLease {
    pub fn new(original: f64, applied: f64) -> Option<Self> {
        if [original, applied]
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return None;
        }
        Some(Self { original, applied })
    }
    pub fn restore(self, current: f64) -> Option<f64> {
        (current.is_finite() && (current - self.applied).abs() < 0.000_001).then_some(self.original)
    }
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn disabled_steps_and_security_order_are_explicit() {
        assert!(Timeouts {
            dim: 0,
            lock: 0,
            off: 0,
            suspend: 0
        }
        .validate()
        .is_ok());
        assert!(Timeouts {
            dim: 300,
            lock: 600,
            off: 900,
            suspend: 1800
        }
        .validate()
        .is_ok());
        assert!(Timeouts {
            dim: 0,
            lock: 600,
            off: 0,
            suspend: 1800
        }
        .validate()
        .is_ok());
        assert_eq!(
            Timeouts {
                dim: 0,
                lock: 0,
                off: 900,
                suspend: 0
            }
            .validate(),
            Err(InvalidTimeouts::LockRequired)
        );
        assert_eq!(
            Timeouts {
                dim: 600,
                lock: 600,
                off: 0,
                suspend: 0
            }
            .validate(),
            Err(InvalidTimeouts::Order)
        );
        assert_eq!(
            Timeouts {
                dim: 0,
                lock: 600,
                off: 900,
                suspend: 700
            }
            .validate(),
            Err(InvalidTimeouts::Order)
        );
        assert_eq!(
            Timeouts {
                dim: 0,
                lock: 90_000,
                off: 0,
                suspend: 0
            }
            .validate(),
            Err(InvalidTimeouts::TooLong)
        );
    }
    #[test]
    fn resume_does_not_overwrite_a_user_brightness_change() {
        let lease = DimLease::new(0.8, 0.2).expect("valid levels");
        assert_eq!(lease.restore(0.2), Some(0.8));
        assert_eq!(lease.restore(0.4), None);
        assert_eq!(lease.restore(f64::NAN), None);
        assert!(DimLease::new(2.0, 0.2).is_none());
    }
}
