//! Shared style entry point. Responsive application widgets arrive in later phases.
use gtk::prelude::*;
pub mod wireless;
use std::{cell::RefCell, rc::Rc};

thread_local! { static STYLE: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) }; }
thread_local! { static PREFERENCES: RefCell<Option<gio::Settings>> = const { RefCell::new(None) }; }

fn install(provider: gtk::CssProvider) {
    if let Some(display) = gtk::gdk::Display::default() {
        STYLE.with(|style| {
            if let Some(old) = style.replace(Some(provider.clone())) {
                gtk::style_context_remove_provider_for_display(&display, &old);
            }
        });
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn reload_style(css: &str) -> Result<(), String> {
    let provider = gtk::CssProvider::new();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let capture = errors.clone();
    let handler = provider
        .connect_parsing_error(move |_, _, error| capture.borrow_mut().push(error.to_string()));
    provider.load_from_string(css);
    provider.disconnect(handler);
    if !errors.borrow().is_empty() {
        return Err(errors.borrow().join("\n"));
    }
    install(provider);
    Ok(())
}
/// Window button layout; matches data/gsettings/90_asena.gschema.override.
pub const DECORATION_LAYOUT: &str = "appmenu:minimize,maximize,close";

/// libadwaita accent palette names accepted by `org.asenalinux.desktop accent-color`.
pub fn accent_css(accent: &str) -> Option<String> {
    asena_core::theme::accent_rgb(accent)
        .map(|_| format!(":root {{ --accent-bg-color: var(--accent-{accent}); }}\n"))
}

thread_local! { static ACCENT: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) }; }
thread_local! { static GLASS: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) }; }

/// Translucent shell material. The compositor blurs what lies behind these
/// surfaces (Dual Kawase in Wayfire). Otherwise the shared stylesheet's fully
/// opaque material applies.
const GLASS_CSS: &str = concat!(
    "window.asena-panel { background-color: alpha(@window_bg_color, 0.78); }\n",
    ".asena-surface { background-color: alpha(@popover_bg_color, 0.8); }\n",
);

/// Glass needs the transparency preference and no high contrast request.
pub fn glass_css(transparency: bool, high_contrast: bool) -> &'static str {
    if transparency && !high_contrast {
        GLASS_CSS
    } else {
        ""
    }
}

fn apply_accent(accent: &str) {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&accent_css(accent).unwrap_or_default());
    ACCENT.with(|slot| {
        if let Some(old) = slot.replace(Some(provider.clone())) {
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
    });
    // Above libadwaita's portal accent, below the shared Asena stylesheet.
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION - 1,
    );
}

fn apply_glass(transparency: bool) {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(glass_css(
        transparency,
        adw::StyleManager::default().is_high_contrast(),
    ));
    GLASS.with(|slot| {
        if let Some(old) = slot.replace(Some(provider.clone())) {
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
    });
    // Above the shared Asena stylesheet, which draws the opaque material.
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
}

pub fn init() -> Result<(), gtk::glib::Error> {
    gio::resources_register_include!("asena.gresource")?;
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/org/asenalinux/style/shell.css");
    install(provider);
    if let Some(native) = gtk::Settings::default() {
        // Asena Odak line icons; missing names fall back to Adwaita. The
        // accent preference below picks the matching folder colors.
        native.set_property("gtk-icon-theme-name", asena_core::theme::ICON_THEME);
        // Asena windows offer minimize, maximize and close, like the panel expects.
        native.set_property("gtk-decoration-layout", DECORATION_LAYOUT);
    }
    if let Ok(settings) = asena_core::settings::load("org.asenalinux.desktop") {
        let update = |settings: &gio::Settings| {
            if let Some(native) = gtk::Settings::default() {
                native.set_property("gtk-font-name", settings.string("font-name").as_str());
                native.set_property(
                    "gtk-xft-dpi",
                    (96.0 * 1024.0 * settings.double("text-scaling-factor")) as i32,
                );
                native.set_property(
                    "gtk-enable-animations",
                    settings.boolean("enable-animations"),
                );
            }
            let manager = adw::StyleManager::default();
            manager.set_color_scheme(match settings.string("color-scheme").as_str() {
                "prefer-dark" => adw::ColorScheme::ForceDark,
                "prefer-light" => adw::ColorScheme::ForceLight,
                _ => adw::ColorScheme::Default,
            });
            let accent = settings.string("accent-color");
            apply_accent(accent.as_str());
            if let Some(native) = gtk::Settings::default() {
                let theme = asena_core::theme::icon_theme(accent.as_str());
                if native.gtk_icon_theme_name().as_deref() != Some(theme.as_str()) {
                    native.set_property("gtk-icon-theme-name", theme);
                }
            }
            apply_glass(settings.boolean("transparency"));
        };
        update(&settings);
        settings.connect_changed(None, move |settings, _| update(settings));
        let preferences = settings.clone();
        adw::StyleManager::default().connect_high_contrast_notify(move |_| {
            apply_glass(preferences.boolean("transparency"));
        });
        PREFERENCES.with(|preferences| {
            preferences.replace(Some(settings));
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{accent_css, glass_css};

    #[test]
    fn accent_names_map_to_libadwaita_palette_only() {
        assert_eq!(
            accent_css("teal").as_deref(),
            Some(":root { --accent-bg-color: var(--accent-teal); }\n")
        );
        assert!(accent_css("slate").is_some());
        assert!(accent_css("teal; color: red").is_none());
        assert!(accent_css("").is_none());
    }

    #[test]
    fn glass_needs_transparency_without_high_contrast() {
        assert!(glass_css(true, false).contains("asena-panel"));
        assert!(glass_css(false, false).is_empty());
        assert!(glass_css(true, true).is_empty());
        // Named colors only, as the shared stylesheet requires.
        assert!(!glass_css(true, false).contains('#'));
    }
}
