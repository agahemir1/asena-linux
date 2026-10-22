use gio::prelude::*;
/// The Asena keys are the source; external desktop changes are mirrored back.
pub fn start(source: &gio::Settings) {
    if std::env::var_os("ASENA_DEV").is_some() {
        tracing::debug!("Skipping native appearance writes in the nested development session");
        return;
    }
    let Ok(target) = asena_core::settings::load("org.gnome.desktop.interface") else {
        tracing::warn!("Desktop interface settings are unavailable");
        return;
    };
    let schema = target.settings_schema();
    for key in [
        "color-scheme",
        "accent-color",
        "font-name",
        "monospace-font-name",
    ] {
        if !schema.as_ref().is_some_and(|schema| schema.has_key(key)) {
            tracing::warn!(key, "Desktop setting is unsupported");
            continue;
        }
        let value = source.string(key);
        if target.string(key) != value {
            if let Err(error) = target.set_string(key, &value) {
                tracing::warn!(%error,key,"Could not mirror appearance setting");
            }
        }
        let desktop = target.clone();
        source.connect_changed(Some(key), move |source, _| {
            let value = source.string(key);
            if desktop.string(key) != value {
                if let Err(error) = desktop.set_string(key, &value) {
                    tracing::warn!(%error,key,"Could not mirror appearance setting");
                }
            }
        });
        let preference = source.clone();
        target.connect_changed(Some(key), move |target, _| {
            let value = target.string(key);
            if preference.string(key) != value {
                if let Err(error) = preference.set_string(key, &value) {
                    tracing::warn!(%error,key,"Could not adopt external appearance setting");
                }
            }
        });
    }
    if schema
        .as_ref()
        .is_some_and(|schema| schema.has_key("text-scaling-factor"))
    {
        let _ = target.set_double("text-scaling-factor", source.double("text-scaling-factor"));
        let target_scale = target.clone();
        source.connect_changed(Some("text-scaling-factor"), move |source, _| {
            let value = source.double("text-scaling-factor");
            if target_scale.double("text-scaling-factor") != value {
                let _ = target_scale.set_double("text-scaling-factor", value);
            }
        });
        let source_scale = source.clone();
        target.connect_changed(Some("text-scaling-factor"), move |target, _| {
            let value = target.double("text-scaling-factor");
            if source_scale.double("text-scaling-factor") != value {
                let _ = source_scale.set_double("text-scaling-factor", value);
            }
        });
    }
    // Derived one way: the icon theme follows the accent so folders match it.
    if schema
        .as_ref()
        .is_some_and(|schema| schema.has_key("icon-theme"))
    {
        let sync_icons = {
            let target = target.clone();
            move |source: &gio::Settings| {
                let theme = asena_core::theme::icon_theme(source.string("accent-color").as_str());
                if target.string("icon-theme") != theme {
                    if let Err(error) = target.set_string("icon-theme", &theme) {
                        tracing::warn!(%error, "Could not set the icon theme");
                    }
                }
            }
        };
        sync_icons(source);
        source.connect_changed(Some("accent-color"), move |source, _| sync_icons(source));
    }
    if !source.boolean("defaults-applied") {
        if schema
            .as_ref()
            .is_some_and(|schema| schema.has_key("clock-format"))
        {
            let _ = target.set_string("clock-format", "24h");
        }
        let _ = source.set_boolean("defaults-applied", true);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mirrors_external_changes_without_resetting_one_time_defaults(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source = asena_core::settings::load("org.asenalinux.desktop")?;
        let target = asena_core::settings::load("org.gnome.desktop.interface")?;
        source.set_string("color-scheme", "prefer-dark")?;
        source.set_string("accent-color", "teal")?;
        source.set_boolean("defaults-applied", false)?;
        start(&source);
        assert_eq!(target.string("color-scheme"), "prefer-dark");
        assert_eq!(target.string("icon-theme"), "Asena");
        source.set_string("accent-color", "purple")?;
        assert_eq!(target.string("icon-theme"), "Asena-purple");
        source.set_string("accent-color", "teal")?;
        assert!(source.boolean("defaults-applied"));
        target.set_string("color-scheme", "prefer-light")?;
        assert_eq!(source.string("color-scheme"), "prefer-light");
        target.set_string("clock-format", "12h")?;
        start(&source);
        assert_eq!(target.string("clock-format"), "12h");
        Ok(())
    }
}
