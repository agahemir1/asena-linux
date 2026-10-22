//! Parser for the Debian 13 wlsunset 0.3 diagnostic contract.
use std::collections::BTreeSet;
#[derive(Default)]
pub struct OutputState {
    failed: BTreeSet<u32>,
    applied: BTreeSet<u32>,
    known: BTreeSet<u32>,
    seen: bool,
}
impl OutputState {
    pub fn line(&mut self, line: &str) {
        if let Some(id) = line
            .strip_prefix("registry: adding output ")
            .and_then(|v| v.parse::<u32>().ok())
        {
            self.known.insert(id);
            self.seen = true;
        }
        if let Some(id) = line
            .strip_prefix("registry: removing output ")
            .and_then(|v| v.parse::<u32>().ok())
        {
            self.known.remove(&id);
            self.failed.remove(&id);
            self.applied.remove(&id);
        }
        if let Some(id) = line
            .strip_prefix("gamma control of output ")
            .and_then(|value| value.strip_suffix(" failed"))
            .and_then(|value| value.parse().ok())
        {
            self.failed.insert(id);
            self.applied.remove(&id);
        }
        if let Some(id) = line
            .strip_prefix("setting temperature on output '")
            .and_then(|value| value.split_once('\'').map(|(id, _)| id))
            .and_then(|value| value.parse::<u32>().ok())
        {
            if !self.failed.contains(&id) {
                self.applied.insert(id);
            }
        }
    }
    pub fn active(&self) -> bool {
        !self.applied.is_empty()
    }
    pub fn unsupported(&self) -> bool {
        (!self.failed.is_empty() || (self.seen && self.known.is_empty())) && self.applied.is_empty()
    }
    pub fn partial(&self) -> bool {
        !self.failed.is_empty() && !self.applied.is_empty()
    }
}
pub fn arguments(temperature: u32) -> Option<Vec<String>> {
    if !(1000..=6500).contains(&temperature) {
        return None;
    }
    Some(vec![
        "wlsunset".into(),
        "-t".into(),
        temperature.to_string(),
        "-T".into(),
        (temperature + 1).to_string(),
        "-S".into(),
        "00:00".into(),
        "-s".into(),
        "00:00".into(),
        "-d".into(),
        "0".into(),
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_outputs_do_not_count_as_applied() {
        let mut state = OutputState::default();
        state.line("gamma control of output 4 failed");
        state.line("setting temperature on output '4' to 3500 K");
        assert!(!state.active());
        assert!(state.unsupported());
        state.line("setting temperature on output '5' to 3500 K");
        assert!(state.active());
        state.line("gamma control of output 5 failed");
        assert!(!state.active());
    }
    #[test]
    fn fixed_schedule_and_temperature_bounds() {
        assert!(arguments(999).is_none());
        assert!(arguments(6501).is_none());
        let args = arguments(3500).unwrap_or_default();
        assert!(args.windows(2).any(|pair| pair == ["-S", "00:00"]));
        assert!(args.windows(2).any(|pair| pair == ["-s", "00:00"]));
    }
}
