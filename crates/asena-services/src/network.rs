//! NetworkManager Wi-Fi snapshots and explicit personal/open connection requests.
use crate::{
    error::ServiceError,
    object_model::{ObjectModel, Properties},
};
use asena_core::i18n::gettext;
use std::collections::{BTreeMap, HashMap};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str, Value};
pub const MANAGER: &str = "/org/freedesktop/NetworkManager";
pub const INTERFACE: &str = "org.freedesktop.NetworkManager";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Security {
    Open,
    Personal,
    Sae,
    Unsupported,
}
#[derive(Clone, Debug)]
pub struct AccessPoint {
    pub id: String,
    pub path: String,
    pub device: String,
    pub ssid: Vec<u8>,
    pub name: String,
    pub strength: u8,
    pub security: Security,
    pub active: bool,
    pub connecting: bool,
}
fn unsigned(properties: &Properties, name: &str) -> u32 {
    properties
        .get(name)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0)
}
fn paths(properties: &Properties, name: &str) -> Vec<OwnedObjectPath> {
    properties
        .get(name)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::try_from(value).ok())
        .unwrap_or_default()
}
pub fn wifi(model: &ObjectModel) -> Option<String> {
    model.with_objects(|objects| {
        let mut devices: Vec<_> = objects
            .iter()
            .filter(|(_, interfaces)| {
                interfaces.contains_key("org.freedesktop.NetworkManager.Device.Wireless")
            })
            .map(|(path, _)| path.to_string())
            .collect();
        devices.sort();
        devices.into_iter().next()
    })
}
pub fn access_points(model: &ObjectModel, device: &str) -> Vec<AccessPoint> {
    model.with_objects(|objects| {
        let Some(device_interfaces) = objects
            .iter()
            .find_map(|(path, interfaces)| (path.as_str() == device).then_some(interfaces))
        else {
            return Vec::new();
        };
        let Some(wireless) =
            device_interfaces.get("org.freedesktop.NetworkManager.Device.Wireless")
        else {
            return Vec::new();
        };
        let allowed = paths(wireless, "AccessPoints");
        let active = wireless
            .get("ActiveAccessPoint")
            .and_then(|v| <&zbus::zvariant::ObjectPath>::try_from(v).ok())
            .map(ToString::to_string);
        let state = device_interfaces
            .get("org.freedesktop.NetworkManager.Device")
            .map(|p| unsigned(p, "State"))
            .unwrap_or(0);
        let mut groups = BTreeMap::<String, AccessPoint>::new();
        for (path, interfaces) in objects {
            if !allowed.contains(path) {
                continue;
            }
            let Some(properties) = interfaces.get("org.freedesktop.NetworkManager.AccessPoint")
            else {
                continue;
            };
            let ssid = properties
                .get("Ssid")
                .and_then(|v| v.try_clone().ok())
                .and_then(|v| Vec::<u8>::try_from(v).ok())
                .unwrap_or_default();
            if ssid.is_empty() || ssid.len() > 32 {
                continue;
            }
            let wpa = unsigned(properties, "WpaFlags") | unsigned(properties, "RsnFlags");
            let flags = unsigned(properties, "Flags");
            let security = classify_security(flags, wpa);
            let id = format!(
                "{}:{wpa:x}:{flags:x}",
                ssid.iter()
                    .flat_map(|byte| {
                        let digits = b"0123456789abcdef";
                        [
                            digits[(byte >> 4) as usize] as char,
                            digits[(byte & 15) as usize] as char,
                        ]
                    })
                    .collect::<String>()
            );
            let strength = properties
                .get("Strength")
                .and_then(|v| u8::try_from(v).ok())
                .unwrap_or(0)
                .min(100);
            let item = AccessPoint {
                id: id.clone(),
                path: path.to_string(),
                device: device.into(),
                name: String::from_utf8_lossy(&ssid).into_owned(),
                ssid,
                strength,
                security,
                active: state == 100 && active.as_deref() == Some(path.as_str()),
                connecting: (40..100).contains(&state) && active.as_deref() == Some(path.as_str()),
            };
            if let Some(previous) = groups.get(&id) {
                if previous.active || (!item.active && previous.strength >= item.strength) {
                    continue;
                }
            }
            groups.insert(id, item);
        }
        let mut result: Vec<_> = groups.into_values().collect();
        result.sort_by(|a, b| {
            b.active
                .cmp(&a.active)
                .then(b.strength.cmp(&a.strength))
                .then(a.id.cmp(&b.id))
        });
        result
    })
}
pub fn classify_security(flags: u32, key_management: u32) -> Security {
    if flags & 1 == 0 && key_management == 0 {
        Security::Open
    } else if key_management & 0x100 != 0 {
        Security::Personal
    } else if key_management & 0x400 != 0 {
        Security::Sae
    } else {
        Security::Unsupported
    }
}
pub fn valid_password(security: Security, password: &str) -> bool {
    match security {
        Security::Open => true,
        Security::Personal => {
            (8..=63).contains(&password.len())
                || (password.len() == 64 && password.bytes().all(|byte| byte.is_ascii_hexdigit()))
        }
        Security::Sae => (1..=63).contains(&password.len()),
        Security::Unsupported => false,
    }
}
pub async fn scan(model: &ObjectModel, device: &str) -> Result<(), ServiceError> {
    model
        .call::<_, ()>(
            device,
            "org.freedesktop.NetworkManager.Device.Wireless",
            "RequestScan",
            &(HashMap::<String, OwnedValue>::new(),),
        )
        .await
}
fn text(value: impl Into<String>) -> OwnedValue {
    OwnedValue::from(Str::from(value.into()))
}
pub async fn connect(
    model: &ObjectModel,
    point: &AccessPoint,
    password: String,
) -> Result<(), ServiceError> {
    if !valid_password(point.security, &password) {
        return Err(gettext("Enter a valid network password.").into());
    }
    let current = access_points(model, &point.device);
    if !current
        .iter()
        .any(|item| item.path == point.path && item.ssid == point.ssid)
    {
        return Err(gettext("The network is no longer available.").into());
    }
    let mut settings = HashMap::<String, Properties>::new();
    settings.insert(
        "connection".into(),
        HashMap::from([
            ("id".into(), text(&point.name)),
            ("type".into(), text("802-11-wireless")),
            ("autoconnect".into(), false.into()),
        ]),
    );
    let ssid = OwnedValue::try_from(Value::from(point.ssid.clone()))
        .map_err(|e| ServiceError(e.to_string()))?;
    settings.insert(
        "802-11-wireless".into(),
        HashMap::from([
            ("ssid".into(), ssid),
            ("mode".into(), text("infrastructure")),
        ]),
    );
    if point.security != Security::Open {
        settings.insert(
            "802-11-wireless-security".into(),
            HashMap::from([
                (
                    "key-mgmt".into(),
                    text(if point.security == Security::Sae {
                        "sae"
                    } else {
                        "wpa-psk"
                    }),
                ),
                ("psk".into(), text(password)),
                ("psk-flags".into(), 0_u32.into()),
            ]),
        );
    }
    settings.insert(
        "ipv4".into(),
        HashMap::from([("method".into(), text("auto"))]),
    );
    settings.insert(
        "ipv6".into(),
        HashMap::from([("method".into(), text("auto"))]),
    );
    let device =
        OwnedObjectPath::try_from(point.device.clone()).map_err(|e| ServiceError(e.to_string()))?;
    let point =
        OwnedObjectPath::try_from(point.path.clone()).map_err(|e| ServiceError(e.to_string()))?;
    // Keep the initial connection transient; credentials stay in NetworkManager.
    let options = HashMap::from([("persist".to_owned(), text("volatile"))]);
    let _: (OwnedObjectPath, OwnedObjectPath, Properties) = model
        .call(
            MANAGER,
            INTERFACE,
            "AddAndActivateConnection2",
            &(settings, device, point, options),
        )
        .await?;
    // The return value means activation started. Device state is the confirmation.
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn security_and_password_boundaries() {
        assert_eq!(classify_security(0, 0), Security::Open);
        assert_eq!(classify_security(1, 0x100), Security::Personal);
        assert_eq!(classify_security(1, 0x400), Security::Sae);
        assert_eq!(classify_security(1, 0x200), Security::Unsupported);
        assert!(!valid_password(Security::Personal, "short"));
        assert!(valid_password(Security::Personal, "password"));
        assert!(valid_password(Security::Personal, &"a".repeat(64)));
        assert!(!valid_password(Security::Personal, &"z".repeat(64)));
    }
}
