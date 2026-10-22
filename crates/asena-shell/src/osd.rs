use asena_core::{
    i18n::gettext,
    layout::{PanelEdge, PanelLayout},
};
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{cell::RefCell, rc::Rc, time::Duration};
#[derive(Clone)]
pub struct Osd {
    window: gtk::ApplicationWindow,
    icon: gtk::Image,
    progress: gtk::ProgressBar,
    label: gtk::Label,
    timer: Rc<RefCell<Option<glib::SourceId>>>,
}
impl Osd {
    pub fn new(app: &adw::Application, layout: PanelLayout) -> Self {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .build();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace(Some("asena-osd"));
        window.set_exclusive_zone(-1);
        window.set_keyboard_mode(KeyboardMode::None);
        let settings = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA).ok();
        position(
            &window,
            settings
                .as_ref()
                .map(PanelLayout::from_settings)
                .unwrap_or(layout),
        );
        if let Some(settings) = settings {
            let weak = window.downgrade();
            settings.connect_changed(None, move |settings, key| {
                if matches!(key, "panel-edge" | "panel-thickness") {
                    if let Some(window) = weak.upgrade() {
                        window.set_visible(false);
                        position(&window, PanelLayout::from_settings(settings));
                    }
                }
            });
        }
        window.add_css_class("asena-surface");
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(24);
        content.set_margin_end(24);
        let icon = gtk::Image::new();
        let progress = gtk::ProgressBar::new();
        progress.set_width_request(160);
        progress.set_valign(gtk::Align::Center);
        let label = gtk::Label::new(None);
        content.append(&icon);
        content.append(&progress);
        content.append(&label);
        window.set_child(Some(&content));
        window.connect_realize(|window| {
            if let Some(surface) = window.surface() {
                surface.set_input_region(&gtk::cairo::Region::create());
            }
        });
        Self {
            window,
            icon,
            progress,
            label,
            timer: Rc::default(),
        }
    }
    pub fn show(&self, kind: &str, value: f64) {
        let icon = match kind {
            "brightness" => "display-brightness-symbolic",
            "microphone" => {
                if value == 0.0 {
                    "microphone-sensitivity-muted-symbolic"
                } else {
                    "microphone-sensitivity-high-symbolic"
                }
            }
            "keyboard" => "input-keyboard-symbolic",
            _ => {
                if value == 0.0 {
                    "audio-volume-muted-symbolic"
                } else {
                    "audio-volume-high-symbolic"
                }
            }
        };
        self.icon.set_icon_name(Some(icon));
        self.progress.set_fraction(value);
        self.progress
            .set_visible(kind != "microphone" && kind != "keyboard");
        self.label
            .set_visible(kind == "microphone" || kind == "keyboard");
        self.label.set_text(&if kind == "microphone" {
            if value == 0.0 {
                gettext("Microphone muted")
            } else {
                gettext("Microphone on")
            }
        } else {
            gettext("Keyboard")
        });
        if let Some(id) = self.timer.borrow_mut().take() {
            id.remove();
        }
        self.window.present();
        let weak = self.window.downgrade();
        let timer = self.timer.clone();
        self.timer.replace(Some(glib::timeout_add_local_once(
            Duration::from_millis(1500),
            move || {
                timer.borrow_mut().take();
                if let Some(window) = weak.upgrade() {
                    window.set_visible(false);
                }
            },
        )));
    }
}

fn position(window: &gtk::ApplicationWindow, layout: PanelLayout) {
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, false);
        window.set_margin(edge, 0);
    }
    match layout.edge {
        PanelEdge::Bottom => {
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, layout.thickness + 24);
        }
        PanelEdge::Top => {
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, layout.thickness + 24);
        }
        PanelEdge::Left | PanelEdge::Right => {
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, 24);
        }
    }
}
