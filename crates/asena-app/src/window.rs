//! Standard window construction and toast overlay management for Asena applications.

use adw::prelude::*;
use std::ops::Deref;

/// Configuration geometry for an Asena window.
#[derive(Debug, Clone)]
pub struct AsenaWindowConfig {
    pub title: String,
    pub icon_name: String,
    pub default_width: i32,
    pub default_height: i32,
    pub min_width: i32,
    pub min_height: i32,
}

impl Default for AsenaWindowConfig {
    fn default() -> Self {
        Self {
            title: "Asena Application".to_string(),
            icon_name: "org.asenalinux.App".to_string(),
            default_width: 800,
            default_height: 550,
            min_width: 360,
            min_height: 300,
        }
    }
}

/// A standard Asena application window with an integrated toast overlay.
#[derive(Clone)]
pub struct AsenaWindow {
    window: adw::ApplicationWindow,
    overlay: adw::ToastOverlay,
}

impl Deref for AsenaWindow {
    type Target = adw::ApplicationWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl AsenaWindow {
    /// Returns a reference to the underlying `adw::ApplicationWindow`.
    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    /// Returns a reference to the `adw::ToastOverlay`.
    pub fn overlay(&self) -> &adw::ToastOverlay {
        &self.overlay
    }

    /// Sets the main content child inside the toast overlay.
    pub fn set_content(&self, widget: &impl IsA<gtk::Widget>) {
        self.overlay.set_child(Some(widget));
    }

    /// Shows a standard toast notification at the bottom of the window.
    pub fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        self.overlay.add_toast(toast);
    }

    /// Shows a toast notification with an action button (e.g., Undo).
    pub fn show_toast_with_action(&self, message: &str, label: &str, action_name: &str) {
        let toast = adw::Toast::new(message);
        toast.set_button_label(Some(label));
        toast.set_action_name(Some(action_name));
        toast.set_timeout(5);
        self.overlay.add_toast(toast);
    }

    /// Presents the window.
    pub fn present(&self) {
        self.window.present();
    }
}

/// Builder for constructing an `AsenaWindow`.
pub struct AsenaWindowBuilder {
    config: AsenaWindowConfig,
    app: adw::Application,
    content: Option<gtk::Widget>,
}

impl AsenaWindowBuilder {
    /// The window icon defaults to the application id, which is also the
    /// application's icon name.
    pub fn new(app: &adw::Application) -> Self {
        let mut config = AsenaWindowConfig::default();
        if let Some(id) = app.application_id() {
            config.icon_name = id.to_string();
        }
        Self {
            config,
            app: app.clone(),
            content: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn icon_name(mut self, icon: impl Into<String>) -> Self {
        self.config.icon_name = icon.into();
        self
    }

    pub fn default_size(mut self, width: i32, height: i32) -> Self {
        self.config.default_width = width;
        self.config.default_height = height;
        self
    }

    pub fn min_size(mut self, width: i32, height: i32) -> Self {
        self.config.min_width = width;
        self.config.min_height = height;
        self
    }

    pub fn content(mut self, content: &impl IsA<gtk::Widget>) -> Self {
        self.content = Some(content.clone().upcast());
        self
    }

    pub fn build(self) -> AsenaWindow {
        let window = adw::ApplicationWindow::builder()
            .application(&self.app)
            .title(&self.config.title)
            .icon_name(&self.config.icon_name)
            .default_width(self.config.default_width)
            .default_height(self.config.default_height)
            .build();

        window.set_size_request(self.config.min_width, self.config.min_height);

        let overlay = adw::ToastOverlay::new();
        if let Some(content) = self.content {
            overlay.set_child(Some(&content));
        }
        window.set_content(Some(&overlay));

        AsenaWindow { window, overlay }
    }
}

/// Honest window for an application that is not ready in this release.
///
/// Used instead of placeholder content so nothing pretends to work.
pub fn unavailable_window(
    app: &adw::Application,
    title: &str,
    icon: &str,
    message: &str,
) -> AsenaWindow {
    let page = adw::StatusPage::builder()
        .icon_name(icon)
        .title(title)
        .description(message)
        .vexpand(true)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));
    AsenaWindowBuilder::new(app)
        .title(title)
        .default_size(480, 420)
        .min_size(360, 320)
        .content(&toolbar)
        .build()
}
