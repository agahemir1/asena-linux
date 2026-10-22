//! Standard state widgets for Asena applications (Empty, Loading, Error).

use crate::i18n::gettext;
use gtk::prelude::*;

/// Helper to create a consistent empty state page.
pub struct EmptyState;

impl EmptyState {
    /// Creates a basic status page with an icon, title, and description.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(icon_name: &str, title: &str, description: &str) -> adw::StatusPage {
        let page = adw::StatusPage::new();
        page.set_icon_name(Some(icon_name));
        page.set_title(title);
        page.set_description(Some(description));
        page
    }

    /// Creates an empty state with an actionable button.
    pub fn with_action<F>(
        icon_name: &str,
        title: &str,
        description: &str,
        action_label: &str,
        on_action: F,
    ) -> adw::StatusPage
    where
        F: Fn() + 'static,
    {
        let page = Self::new(icon_name, title, description);
        let button = gtk::Button::with_label(action_label);
        button.add_css_class("suggested-action");
        button.add_css_class("pill");
        button.set_halign(gtk::Align::Center);
        button.connect_clicked(move |_| on_action());
        page.set_child(Some(&button));
        page
    }
}

/// Helper to create a consistent loading view.
pub struct LoadingState;

impl LoadingState {
    /// Creates a centered box containing an active spinner and a loading label.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(message: &str) -> gtk::Box {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 16);
        container.set_valign(gtk::Align::Center);
        container.set_halign(gtk::Align::Center);

        let spinner = gtk::Spinner::new();
        spinner.set_spinning(true);
        spinner.set_size_request(48, 48);

        let label = gtk::Label::new(Some(message));
        label.add_css_class("title-4");
        label.add_css_class("dim-label");

        container.append(&spinner);
        container.append(&label);
        container
    }
}

/// Helper to create a consistent error view.
pub struct ErrorState;

impl ErrorState {
    /// Creates an error display with an error icon, heading, and description.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(title: &str, error_message: &str) -> gtk::Box {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        container.set_valign(gtk::Align::Center);
        container.set_halign(gtk::Align::Center);
        container.set_margin_start(24);
        container.set_margin_end(24);

        let icon = gtk::Image::from_icon_name("dialog-error-symbolic");
        icon.set_pixel_size(64);
        icon.add_css_class("error");

        let heading = gtk::Label::new(Some(title));
        heading.add_css_class("title-2");

        let message = gtk::Label::new(Some(error_message));
        message.add_css_class("body");
        message.add_css_class("dim-label");
        message.set_wrap(true);
        message.set_max_width_chars(45);
        message.set_justify(gtk::Justification::Center);

        container.append(&icon);
        container.append(&heading);
        container.append(&message);
        container
    }

    /// Creates an error display with a retry button.
    pub fn with_retry<F>(title: &str, error_message: &str, retry: F) -> gtk::Box
    where
        F: Fn() + 'static,
    {
        let container = Self::new(title, error_message);
        let button = gtk::Button::with_label(&gettext("Try Again"));
        button.add_css_class("suggested-action");
        button.add_css_class("pill");
        button.set_halign(gtk::Align::Center);
        button.set_margin_top(12);
        button.connect_clicked(move |_| retry());
        container.append(&button);
        container
    }
}
