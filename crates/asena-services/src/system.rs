//! Shared models for the initial power/network service integration.
use crate::property_model::{Endpoint, PropertyModel};
use asena_core::i18n::gettext;

#[derive(Clone)]
pub struct SystemServices {
    pub user: crate::user::UserModel,
    pub mpris: crate::mpris::MprisModel,
    pub tray: crate::tray::TrayModel,
    pub daemon: PropertyModel,
    pub network_objects: crate::object_model::ObjectModel,
    pub bluetooth: crate::object_model::ObjectModel,
    pub audio: crate::audio::AudioModel,
    pub brightness: crate::brightness::BrightnessModel,
    pub network: PropertyModel,
    pub battery: PropertyModel,
    pub profiles: PropertyModel,
}
impl Default for SystemServices {
    fn default() -> Self {
        Self {
            user: Default::default(),
            mpris: Default::default(),
            tray: Default::default(),
            daemon: PropertyModel::new(Endpoint {
                bus: crate::property_model::Bus::Session,
                name: asena_core::dbus::DAEMON_ID.into(),
                path: asena_core::dbus::DAEMON_PATH.into(),
                interface: asena_core::dbus::DAEMON_ID.into(),
            }),
            network_objects: crate::object_model::ObjectModel::new(Endpoint::system(
                "org.freedesktop.NetworkManager",
                "/org/freedesktop",
                "org.freedesktop.DBus.ObjectManager",
            )),
            bluetooth: crate::object_model::ObjectModel::new(Endpoint::system(
                "org.bluez",
                "/",
                "org.freedesktop.DBus.ObjectManager",
            )),
            audio: Default::default(),
            brightness: Default::default(),
            network: PropertyModel::new(Endpoint::system(
                "org.freedesktop.NetworkManager",
                "/org/freedesktop/NetworkManager",
                "org.freedesktop.NetworkManager",
            )),
            battery: PropertyModel::new(Endpoint::system(
                "org.freedesktop.UPower",
                "/org/freedesktop/UPower/devices/DisplayDevice",
                "org.freedesktop.UPower.Device",
            )),
            profiles: PropertyModel::new(Endpoint::system(
                "org.freedesktop.UPower.PowerProfiles",
                "/org/freedesktop/UPower/PowerProfiles",
                "org.freedesktop.UPower.PowerProfiles",
            )),
        }
    }
}
impl SystemServices {
    pub fn start_preferences(&self) {
        self.user.start();
        self.daemon.start();
        self.network_objects.start();
        self.bluetooth.start();
        self.network.start();
        self.battery.start();
        self.profiles.start();
        if let Err(error) = self.audio.start() {
            tracing::warn!(%error,"Audio unavailable");
        }
        if let Err(error) = self.brightness.start() {
            tracing::warn!(%error,"Backlight unavailable");
        }
    }
    pub fn start(&self) {
        self.user.start();
        self.mpris.start();
        self.tray.start();
        self.daemon.start();
        self.network_objects.start();
        self.bluetooth.start();
        if let Err(error) = self.brightness.start() {
            tracing::warn!(%error,"Could not start the brightness model");
        }
        if let Err(error) = self.audio.start() {
            tracing::warn!(%error,"Could not start the audio model");
        }
        self.network.start();
        self.battery.start();
        self.profiles.start();
    }
}
pub fn battery_percentage(model: &PropertyModel) -> Option<f64> {
    let battery = model
        .with_values(|values| values.get("Type").and_then(|v| u32::try_from(v).ok()))
        == Some(2);
    if !model.available() || model.boolean("IsPresent") != Some(true) || !battery {
        return None;
    }
    model
        .number("Percentage")
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
}
/// Snapshot of NetworkManager state for status icons and the active Wi-Fi name.
pub fn network_state(
    manager: &PropertyModel,
    objects: &crate::object_model::ObjectModel,
) -> (asena_core::status::NetworkState, Option<String>) {
    let device = crate::network::wifi(objects);
    let active = device.as_deref().and_then(|device| {
        crate::network::access_points(objects, device)
            .into_iter()
            .find(|point| point.active)
    });
    let state = asena_core::status::NetworkState {
        available: manager.available(),
        state: manager.unsigned("State").unwrap_or(0),
        primary_type: manager.string("PrimaryConnectionType").unwrap_or_default(),
        wifi_device: device.is_some(),
        wifi_enabled: manager.boolean("WirelessEnabled").unwrap_or(false),
        wifi_strength: active.as_ref().map(|point| point.strength),
    };
    (state, active.map(|point| point.name))
}
/// UPower display device state for status icons.
pub fn battery_state(model: &PropertyModel) -> asena_core::status::BatteryState {
    asena_core::status::BatteryState::from_upower(model.unsigned("State").unwrap_or(0))
}
pub fn profile_ids(model: &PropertyModel) -> Vec<String> {
    model.with_values(|values| {
        values
            .get("Profiles")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| {
                Vec::<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>::try_from(v)
                    .ok()
            })
            .unwrap_or_default()
            .iter()
            .filter_map(|profile| {
                profile
                    .get("Profile")
                    .and_then(|v| <&str>::try_from(v).ok())
                    .map(ToOwned::to_owned)
            })
            .collect()
    })
}
pub fn profile_label(id: &str) -> String {
    match id {
        "power-saver" => gettext("Power saver"),
        "balanced" => gettext("Balanced"),
        "performance" => gettext("Performance"),
        _ => id.into(),
    }
}
