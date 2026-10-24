use gio::prelude::*;
use thiserror::Error;

use crate::i18n::gettext;

pub const SHELL_SCHEMA: &str = "org.asenalinux.shell";

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("{message}", message = self.message())]
    NoSource,
    #[error("{message}: {0}", message = self.message())]
    MissingSchema(String),
    #[error("{message}: {0}", message = self.message())]
    MissingPath(String),
}

impl SettingsError {
    // Keep calls in ordinary Rust code: Debian's C extractor ignores #[...] lines.
    fn message(&self) -> String {
        match self {
            Self::NoSource => gettext("No settings schemas are available. Run scripts/compile-data.sh and set GSETTINGS_SCHEMA_DIR."),
            Self::MissingSchema(_) => gettext("The settings schema is missing. Run scripts/compile-data.sh and set GSETTINGS_SCHEMA_DIR to target/schemas."),
            Self::MissingPath(_) => gettext("The settings schema has no fixed path."),
        }
    }
}

/// Look up the schema before constructing Settings, avoiding GLib's fatal error.
pub fn load(id: &str) -> Result<gio::Settings, SettingsError> {
    let source = gio::SettingsSchemaSource::default().ok_or(SettingsError::NoSource)?;
    load_from(&source, id, None)
}

pub fn load_from(
    source: &gio::SettingsSchemaSource,
    id: &str,
    backend: Option<&gio::SettingsBackend>,
) -> Result<gio::Settings, SettingsError> {
    let schema = source
        .lookup(id, true)
        .ok_or_else(|| SettingsError::MissingSchema(id.to_owned()))?;
    if schema.path().is_none() {
        return Err(SettingsError::MissingPath(id.to_owned()));
    }
    Ok(gio::Settings::new_full(&schema, backend, None))
}

/// Keys "Reset Asena settings" keeps: the layouts decide how the password is
/// typed at login and on the lock screen, and backups must not stop silently.
const KEPT_ON_RESET: &[(&str, &str)] = &[
    ("org.asenalinux.input", "keyboard-layouts"),
    ("org.asenalinux.input", "keyboard-toggle"),
    ("org.asenalinux.backup", "destination"),
    ("org.asenalinux.backup", "automatic"),
    ("org.asenalinux.backup", "last-backup"),
];

/// Returns every Asena preference of the user, except the keyboard layouts,
/// to its default. Returns the ids of the schemas that were reset.
pub fn reset_all(
    source: &gio::SettingsSchemaSource,
    backend: Option<&gio::SettingsBackend>,
) -> Vec<String> {
    let (ids, _) = source.list_schemas(true);
    let mut ids: Vec<String> = ids
        .into_iter()
        .map(|id| id.to_string())
        .filter(|id| id.starts_with("org.asenalinux."))
        .collect();
    ids.sort();
    ids.retain(|id| {
        let Ok(settings) = load_from(source, id, backend) else {
            return false;
        };
        let Some(schema) = source.lookup(id, true) else {
            return false;
        };
        // One change notification per schema instead of one per key.
        settings.delay();
        for key in schema.list_keys() {
            if !KEPT_ON_RESET
                .iter()
                .any(|(schema, kept)| schema == id && *kept == key.as_str())
            {
                settings.reset(&key);
            }
        }
        settings.apply();
        true
    });
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_restores_defaults_but_keeps_keyboard_layouts() -> Result<(), Box<dyn std::error::Error>>
    {
        let source = gio::SettingsSchemaSource::from_directory(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/schemas"),
            None,
            false,
        )?;
        let backend = gio::memory_settings_backend_new();
        let shell = load_from(&source, SHELL_SCHEMA, Some(&backend))?;
        let input = load_from(&source, "org.asenalinux.input", Some(&backend))?;
        shell.set_string("panel-edge", "top")?;
        input.set_strv("keyboard-layouts", ["us"])?;
        input.set_int("repeat-rate", 60)?;
        let reset = reset_all(&source, Some(&backend));
        assert!(reset.iter().any(|id| id == SHELL_SCHEMA));
        assert!(reset.iter().all(|id| id.starts_with("org.asenalinux.")));
        assert_eq!(shell.string("panel-edge"), "bottom");
        assert_eq!(input.int("repeat-rate"), 25);
        assert_eq!(input.strv("keyboard-layouts")[0], "us");
        Ok(())
    }

    #[test]
    fn missing_schema_is_recoverable() {
        assert!(matches!(
            load("org.asenalinux.Nonexistent"),
            Err(SettingsError::MissingSchema(_)) | Err(SettingsError::NoSource)
        ));
    }

    #[test]
    fn compiled_schema_has_expected_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let source = gio::SettingsSchemaSource::from_directory(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/schemas"),
            None,
            false,
        )?;
        let backend = gio::memory_settings_backend_new();
        let settings = load_from(&source, SHELL_SCHEMA, Some(&backend))?;
        assert!(settings.strv("pinned-apps").is_empty());
        settings.set_strv("pinned-apps", ["org.gnome.Nautilus.desktop"])?;
        assert_eq!(
            settings.strv("pinned-apps")[0],
            "org.gnome.Nautilus.desktop"
        );
        Ok(())
    }
}
