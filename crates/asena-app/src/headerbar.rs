//! Standard header bar builder for Asena applications.

use crate::i18n::gettext;
use adw::prelude::*;

/// Builder for constructing consistent `adw::HeaderBar` widgets.
#[derive(Default)]
pub struct AsenaHeaderBarBuilder {
    title: Option<String>,
    title_widget: Option<gtk::Widget>,
    show_back_button: bool,
    on_back: Option<Box<dyn Fn() + 'static>>,
    search_button: Option<gtk::ToggleButton>,
    menu_model: Option<gio::MenuModel>,
    start_widgets: Vec<gtk::Widget>,
    end_widgets: Vec<gtk::Widget>,
}

impl AsenaHeaderBarBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn title_widget(mut self, widget: impl IsA<gtk::Widget>) -> Self {
        self.title_widget = Some(widget.upcast());
        self
    }

    pub fn back_button<F>(mut self, on_back: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.show_back_button = true;
        self.on_back = Some(Box::new(on_back));
        self
    }

    pub fn search_toggle(mut self, toggle: &gtk::ToggleButton) -> Self {
        toggle.set_icon_name("edit-find-symbolic");
        toggle.set_tooltip_text(Some(&gettext("Search")));
        self.search_button = Some(toggle.clone());
        self
    }

    pub fn primary_menu(mut self, menu_model: &gio::MenuModel) -> Self {
        self.menu_model = Some(menu_model.clone());
        self
    }

    pub fn pack_start(mut self, widget: &impl IsA<gtk::Widget>) -> Self {
        self.start_widgets.push(widget.clone().upcast());
        self
    }

    pub fn pack_end(mut self, widget: &impl IsA<gtk::Widget>) -> Self {
        self.end_widgets.push(widget.clone().upcast());
        self
    }

    pub fn build(self) -> adw::HeaderBar {
        let bar = adw::HeaderBar::new();

        if self.show_back_button {
            let back_btn = gtk::Button::from_icon_name("go-previous-symbolic");
            back_btn.set_tooltip_text(Some(&gettext("Back")));
            if let Some(on_back) = self.on_back {
                back_btn.connect_clicked(move |_| on_back());
            }
            bar.pack_start(&back_btn);
        }

        for widget in self.start_widgets {
            bar.pack_start(&widget);
        }

        if let Some(title_widget) = self.title_widget {
            bar.set_title_widget(Some(&title_widget));
        } else if let Some(title) = self.title {
            let label = gtk::Label::new(Some(&title));
            label.add_css_class("title");
            bar.set_title_widget(Some(&label));
        }

        if let Some(search_btn) = self.search_button {
            bar.pack_end(&search_btn);
        }

        for widget in self.end_widgets {
            bar.pack_end(&widget);
        }

        if let Some(model) = self.menu_model {
            let menu_btn = gtk::MenuButton::new();
            menu_btn.set_icon_name("open-menu-symbolic");
            menu_btn.set_tooltip_text(Some(&gettext("Main Menu")));
            menu_btn.set_menu_model(Some(&model));
            bar.pack_end(&menu_btn);
        }

        bar
    }
}
