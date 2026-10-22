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

#[cfg(test)]
mod tests {
    use super::*;
    use gio::prelude::*;

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
