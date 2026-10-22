//! # Asena Application Framework
//!
//! Unified application framework providing standard windows, header bars,
//! toast notifications, empty/loading/error states, dialogs, and background task execution
//! for all built-in and third-party Asena Desktop applications.

pub mod application;
pub mod dialogs;
pub mod headerbar;
pub mod i18n;
pub mod states;
pub mod storage;
pub mod task;
pub mod window;

pub mod prelude {
    pub use crate::{
        application::AsenaApplication,
        dialogs::{confirm_dialog, show_about_dialog, show_error_dialog, AppMetadata},
        headerbar::AsenaHeaderBarBuilder,
        i18n::{gettext, ngettext},
        states::{EmptyState, ErrorState, LoadingState},
        task::spawn_background,
        window::{unavailable_window, AsenaWindow, AsenaWindowBuilder, AsenaWindowConfig},
    };
    pub use adw::prelude::*;
}

#[cfg(test)]
mod tests {
    use crate::dialogs::AppMetadata;
    use crate::window::AsenaWindowConfig;

    #[test]
    fn test_app_metadata_defaults() {
        let meta = AppMetadata::default();
        assert_eq!(meta.app_id, "org.asenalinux.App");
        assert_eq!(meta.version, env!("CARGO_PKG_VERSION"));
        assert!(meta.copyright.contains("Asena"));
        let named = AppMetadata::new("org.asenalinux.Files", "Files".into(), String::new());
        assert_eq!(named.app_id, "org.asenalinux.Files");
        assert_eq!(named.version, meta.version);
    }

    #[test]
    fn test_window_config_geometry() {
        let config = AsenaWindowConfig::default();
        assert!(config.min_width <= config.default_width);
        assert!(config.min_height <= config.default_height);
        assert!(config.min_width >= 320);
        assert!(config.min_height >= 240);
    }
}
