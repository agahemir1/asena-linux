//! Pure status icon selection shared by the panel and quick settings.
//!
//! Every function returns `None` when the backing service is unavailable so the
//! caller can hide the indicator instead of showing a guessed state.

/// NetworkManager global states (`NMState`).
pub const NM_STATE_CONNECTING: u32 = 40;
pub const NM_STATE_CONNECTED_LOCAL: u32 = 50;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkState {
    pub available: bool,
    /// `NMState` of the manager.
    pub state: u32,
    /// `PrimaryConnectionType`, for example `802-11-wireless` or `802-3-ethernet`.
    pub primary_type: String,
    pub wifi_device: bool,
    pub wifi_enabled: bool,
    /// Strength of the active access point, 0–100.
    pub wifi_strength: Option<u8>,
}

pub fn wifi_signal_icon(strength: u8) -> &'static str {
    match strength {
        75.. => "network-wireless-signal-excellent-symbolic",
        50..=74 => "network-wireless-signal-good-symbolic",
        25..=49 => "network-wireless-signal-ok-symbolic",
        _ => "network-wireless-signal-weak-symbolic",
    }
}

pub fn network_icon(network: &NetworkState) -> Option<&'static str> {
    if !network.available {
        return None;
    }
    let connected = network.state >= NM_STATE_CONNECTED_LOCAL;
    match network.primary_type.as_str() {
        "802-3-ethernet" if connected => return Some("network-wired-symbolic"),
        "802-11-wireless" if connected => {
            return Some(wifi_signal_icon(network.wifi_strength.unwrap_or(0)))
        }
        _ if connected => return Some("network-wired-symbolic"),
        _ => {}
    }
    if network.state == NM_STATE_CONNECTING {
        return Some(if network.wifi_device && network.wifi_enabled {
            "network-wireless-acquiring-symbolic"
        } else {
            "network-wired-acquiring-symbolic"
        });
    }
    if network.wifi_device && !network.wifi_enabled {
        return Some("network-wireless-disabled-symbolic");
    }
    Some("network-offline-symbolic")
}

pub fn volume_icon(available: bool, level: f64, muted: bool) -> Option<&'static str> {
    if !available {
        return None;
    }
    let percent = (level * 100.0).round();
    Some(if muted || percent <= 0.0 {
        "audio-volume-muted-symbolic"
    } else if percent < 34.0 {
        "audio-volume-low-symbolic"
    } else if percent < 67.0 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    })
}

/// UPower device states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    Other,
}

impl BatteryState {
    pub fn from_upower(state: u32) -> Self {
        match state {
            1 | 5 => Self::Charging,
            2 | 6 => Self::Discharging,
            4 => Self::Full,
            _ => Self::Other,
        }
    }
}

pub fn battery_icon(percentage: Option<f64>, state: BatteryState) -> Option<String> {
    let value = percentage
        .filter(|value| value.is_finite())?
        .clamp(0.0, 100.0);
    let level = ((value / 10.0).round() * 10.0) as u32;
    Some(match state {
        BatteryState::Full => "battery-level-100-charged-symbolic".to_owned(),
        BatteryState::Charging => format!("battery-level-{level}-charging-symbolic"),
        _ => format!("battery-level-{level}-symbolic"),
    })
}

pub fn bluetooth_icon(adapter: bool, powered: bool, connected: usize) -> Option<&'static str> {
    if !adapter || !powered {
        return None;
    }
    Some(if connected > 0 {
        "bluetooth-connected-symbolic"
    } else {
        "bluetooth-active-symbolic"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network(state: u32, primary: &str) -> NetworkState {
        NetworkState {
            available: true,
            state,
            primary_type: primary.into(),
            wifi_device: true,
            wifi_enabled: true,
            wifi_strength: Some(60),
        }
    }

    #[test]
    fn network_reports_connection_kind_and_strength() {
        assert_eq!(network_icon(&NetworkState::default()), None);
        assert_eq!(
            network_icon(&network(70, "802-3-ethernet")),
            Some("network-wired-symbolic")
        );
        assert_eq!(
            network_icon(&network(70, "802-11-wireless")),
            Some("network-wireless-signal-good-symbolic")
        );
        assert_eq!(
            network_icon(&network(40, "")),
            Some("network-wireless-acquiring-symbolic")
        );
        let mut disabled = network(20, "");
        disabled.wifi_enabled = false;
        assert_eq!(
            network_icon(&disabled),
            Some("network-wireless-disabled-symbolic")
        );
        let mut wired_only = network(20, "");
        wired_only.wifi_device = false;
        assert_eq!(network_icon(&wired_only), Some("network-offline-symbolic"));
        assert_eq!(
            wifi_signal_icon(100),
            "network-wireless-signal-excellent-symbolic"
        );
        assert_eq!(
            wifi_signal_icon(10),
            "network-wireless-signal-weak-symbolic"
        );
    }

    #[test]
    fn volume_levels_and_mute() {
        assert_eq!(volume_icon(false, 0.5, false), None);
        assert_eq!(
            volume_icon(true, 0.5, true),
            Some("audio-volume-muted-symbolic")
        );
        assert_eq!(
            volume_icon(true, 0.0, false),
            Some("audio-volume-muted-symbolic")
        );
        assert_eq!(
            volume_icon(true, 0.2, false),
            Some("audio-volume-low-symbolic")
        );
        assert_eq!(
            volume_icon(true, 0.5, false),
            Some("audio-volume-medium-symbolic")
        );
        assert_eq!(
            volume_icon(true, 0.9, false),
            Some("audio-volume-high-symbolic")
        );
    }

    #[test]
    fn battery_rounds_to_available_icons() {
        assert_eq!(battery_icon(None, BatteryState::Discharging), None);
        assert_eq!(
            battery_icon(Some(f64::NAN), BatteryState::Discharging),
            None
        );
        assert_eq!(
            battery_icon(Some(46.0), BatteryState::Discharging).as_deref(),
            Some("battery-level-50-symbolic")
        );
        assert_eq!(
            battery_icon(Some(3.0), BatteryState::Charging).as_deref(),
            Some("battery-level-0-charging-symbolic")
        );
        assert_eq!(
            battery_icon(Some(100.0), BatteryState::Full).as_deref(),
            Some("battery-level-100-charged-symbolic")
        );
        assert_eq!(BatteryState::from_upower(1), BatteryState::Charging);
        assert_eq!(BatteryState::from_upower(4), BatteryState::Full);
    }

    #[test]
    fn bluetooth_hidden_when_off() {
        assert_eq!(bluetooth_icon(false, true, 0), None);
        assert_eq!(bluetooth_icon(true, false, 0), None);
        assert_eq!(
            bluetooth_icon(true, true, 0),
            Some("bluetooth-active-symbolic")
        );
        assert_eq!(
            bluetooth_icon(true, true, 2),
            Some("bluetooth-connected-symbolic")
        );
    }
}
