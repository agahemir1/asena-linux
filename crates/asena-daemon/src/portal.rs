//! xdg-desktop-portal Settings backend for the appearance namespace. Sandboxed
//! apps read the color scheme and accent color from the Asena preferences;
//! every other key falls through to the next backend (gtk), as configured in
//! data/portals/asena-portals.conf.
use gio::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use zbus::zvariant::{OwnedValue, StructureBuilder, Value};

pub const NAME: &str = "org.freedesktop.impl.portal.desktop.asena";
pub const PATH: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.impl.portal.Settings";
const APPEARANCE: &str = "org.freedesktop.appearance";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    /// 0 no preference, 1 dark, 2 light.
    pub color_scheme: u32,
    /// sRGB channels in 0..=1.
    pub accent: Option<(f64, f64, f64)>,
}

impl Appearance {
    pub fn new(color_scheme: &str, accent_name: &str) -> Self {
        Self {
            color_scheme: match color_scheme {
                "prefer-dark" => 1,
                "prefer-light" => 2,
                _ => 0,
            },
            accent: accent(accent_name),
        }
    }

    fn from_settings(settings: &gio::Settings) -> Self {
        Self::new(
            settings.string("color-scheme").as_str(),
            settings.string("accent-color").as_str(),
        )
    }

    fn value(&self, key: &str) -> Option<Value<'static>> {
        match key {
            "color-scheme" => Some(Value::from(self.color_scheme)),
            "accent-color" => {
                let (red, green, blue) = self.accent?;
                StructureBuilder::new()
                    .add_field(red)
                    .add_field(green)
                    .add_field(blue)
                    .build()
                    .ok()
                    .map(Value::from)
            }
            _ => None,
        }
    }

    fn values(&self) -> HashMap<String, OwnedValue> {
        ["color-scheme", "accent-color"]
            .into_iter()
            .filter_map(|key| {
                let value = OwnedValue::try_from(self.value(key)?).ok()?;
                Some((key.to_owned(), value))
            })
            .collect()
    }
}

pub fn accent(name: &str) -> Option<(f64, f64, f64)> {
    let rgb = asena_core::theme::accent_rgb(name)?;
    let channel = |shift: u32| f64::from((rgb >> shift) & 0xff) / 255.0;
    Some((channel(16), channel(8), channel(0)))
}

/// An empty list or an empty pattern matches everything; a pattern may end
/// with `*` to match a namespace prefix.
fn requested(namespace: &str, patterns: &[String]) -> bool {
    patterns.is_empty()
        || patterns.iter().any(|pattern| {
            pattern.is_empty()
                || pattern == namespace
                || pattern
                    .strip_suffix('*')
                    .is_some_and(|prefix| namespace.starts_with(prefix))
        })
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.portal.Error")]
enum PortalError {
    #[zbus(error)]
    ZBus(zbus::Error),
    NotFound(String),
}

pub struct SettingsPortal {
    state: Arc<Mutex<Appearance>>,
}

impl SettingsPortal {
    fn current(&self) -> Appearance {
        self.state
            .lock()
            .map(|state| *state)
            .unwrap_or_else(|poisoned| *poisoned.into_inner())
    }
}

#[zbus::interface(name = "org.freedesktop.impl.portal.Settings")]
impl SettingsPortal {
    fn read_all(&self, namespaces: Vec<String>) -> HashMap<String, HashMap<String, OwnedValue>> {
        let mut all = HashMap::new();
        if requested(APPEARANCE, &namespaces) {
            all.insert(APPEARANCE.to_owned(), self.current().values());
        }
        all
    }

    fn read(&self, namespace: &str, key: &str) -> Result<OwnedValue, PortalError> {
        let missing = || PortalError::NotFound("Requested setting not found".into());
        if namespace != APPEARANCE {
            return Err(missing());
        }
        let value = self.current().value(key).ok_or_else(missing)?;
        OwnedValue::try_from(value).map_err(|error| PortalError::ZBus(error.into()))
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        2
    }
}

/// The interface to serve at `PATH`, and the shared state `watch` updates.
pub fn create(desktop: &gio::Settings) -> (SettingsPortal, Arc<Mutex<Appearance>>) {
    let state = Arc::new(Mutex::new(Appearance::from_settings(desktop)));
    (
        SettingsPortal {
            state: state.clone(),
        },
        state,
    )
}

/// Owns the backend name and reports appearance changes. A missing name only
/// means sandboxed apps keep reading the gtk backend.
pub async fn start(
    connection: &zbus::Connection,
    desktop: &gio::Settings,
    state: Arc<Mutex<Appearance>>,
) {
    if let Err(error) = asena_core::dbus::claim_name(connection, NAME).await {
        tracing::warn!(%error, "Could not own the portal settings name");
        return;
    }
    let connection = connection.clone();
    desktop.connect_changed(None, move |settings, key| {
        if !matches!(key, "color-scheme" | "accent-color") {
            return;
        }
        let next = Appearance::from_settings(settings);
        let previous = match state.lock() {
            Ok(mut current) => std::mem::replace(&mut *current, next),
            Err(_) => return,
        };
        let Some(value) = next.value(key) else {
            return;
        };
        if previous.value(key) == Some(value.clone()) {
            return;
        }
        let connection = connection.clone();
        let key = key.to_owned();
        glib::spawn_future_local(async move {
            if let Err(error) = connection
                .emit_signal(
                    None::<&str>,
                    PATH,
                    INTERFACE,
                    "SettingChanged",
                    &(APPEARANCE, key.as_str(), value),
                )
                .await
            {
                tracing::warn!(%error, "Could not report a portal setting change");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_scheme_follows_the_portal_values() {
        assert_eq!(Appearance::new("default", "blue").color_scheme, 0);
        assert_eq!(Appearance::new("prefer-dark", "blue").color_scheme, 1);
        assert_eq!(Appearance::new("prefer-light", "blue").color_scheme, 2);
    }

    #[test]
    fn accents_are_the_libadwaita_palette() {
        assert_eq!(
            accent("blue"),
            Some((
                0x35 as f64 / 255.0,
                0x84 as f64 / 255.0,
                0xe4 as f64 / 255.0
            ))
        );
        assert_eq!(
            accent("purple"),
            Some((
                0x91 as f64 / 255.0,
                0x41 as f64 / 255.0,
                0xac as f64 / 255.0
            ))
        );
        assert!(asena_core::theme::ACCENTS
            .iter()
            .all(|(name, _)| accent(name).is_some()));
        assert_eq!(accent("magenta"), None);
    }

    #[test]
    fn namespaces_match_exactly_or_by_prefix() {
        let patterns = |list: &[&str]| list.iter().map(|item| item.to_string()).collect::<Vec<_>>();
        assert!(requested(APPEARANCE, &[]));
        assert!(requested(APPEARANCE, &patterns(&[""])));
        assert!(requested(
            APPEARANCE,
            &patterns(&["org.freedesktop.appearance"])
        ));
        assert!(requested(APPEARANCE, &patterns(&["org.freedesktop.*"])));
        assert!(!requested(APPEARANCE, &patterns(&["org.gnome.*"])));
        assert!(!requested(
            APPEARANCE,
            &patterns(&["org.freedesktop.appearance.extra"])
        ));
    }

    #[test]
    fn reads_serve_only_known_appearance_keys() {
        let portal = SettingsPortal {
            state: Arc::new(Mutex::new(Appearance::new("prefer-dark", "teal"))),
        };
        assert_eq!(
            portal.read(APPEARANCE, "color-scheme").ok(),
            OwnedValue::try_from(Value::from(1u32)).ok()
        );
        assert!(portal.read(APPEARANCE, "accent-color").is_ok());
        assert!(portal.read(APPEARANCE, "contrast").is_err());
        assert!(portal
            .read("org.gnome.desktop.interface", "gtk-theme")
            .is_err());
        let all = portal.read_all(vec!["org.freedesktop.*".into()]);
        assert_eq!(all.get(APPEARANCE).map(HashMap::len), Some(2));
        assert!(portal.read_all(vec!["org.gnome.*".into()]).is_empty());
    }

    #[test]
    fn an_unknown_accent_is_left_to_the_next_backend() {
        let appearance = Appearance::new("default", "magenta");
        assert!(appearance.value("accent-color").is_none());
        assert_eq!(appearance.values().len(), 1);
    }
}
