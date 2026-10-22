//! Media card for the sidebar, driven by the MPRIS model.
use asena_core::i18n::gettext;
use asena_services::mpris::MprisModel;
use gtk::prelude::*;

fn control(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.add_css_class("flat");
    button.add_css_class("circular");
    button.set_tooltip_text(Some(tooltip));
    button
}

pub fn create(model: &MprisModel) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&header);
    let art = gtk::Box::new(gtk::Orientation::Vertical, 0);
    art.add_css_class("asena-media-art");
    art.set_overflow(gtk::Overflow::Hidden);
    art.set_valign(gtk::Align::Center);
    header.append(&art);
    let placeholder = gtk::Image::from_icon_name("audio-x-generic-symbolic");
    placeholder.set_pixel_size(28);
    placeholder.set_size_request(56, 56);
    placeholder.add_css_class("asena-media-placeholder");
    header.prepend(&placeholder);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    header.append(&text);
    let source = gtk::Label::new(None);
    source.set_xalign(0.0);
    source.add_css_class("caption");
    source.add_css_class("dim-label");
    text.append(&source);
    let title = gtk::Label::new(None);
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("heading");
    text.append(&title);
    let artist = gtk::Label::new(None);
    artist.set_xalign(0.0);
    artist.set_ellipsize(gtk::pango::EllipsizeMode::End);
    artist.add_css_class("dim-label");
    text.append(&artist);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    controls.set_halign(gtk::Align::Center);
    let previous = control("media-skip-backward-symbolic", &gettext("Previous"));
    let toggle = control("media-playback-start-symbolic", &gettext("Play or pause"));
    toggle.remove_css_class("flat");
    toggle.add_css_class("asena-media-toggle");
    let next = control("media-skip-forward-symbolic", &gettext("Next"));
    for button in [&previous, &toggle, &next] {
        controls.append(button);
    }
    content.append(&controls);
    for (button, method) in [
        (previous.clone(), "Previous"),
        (toggle.clone(), "PlayPause"),
        (next.clone(), "Next"),
    ] {
        let model = model.clone();
        button.connect_clicked(move |_| {
            let model = model.clone();
            glib::spawn_future_local(async move {
                if let Err(error) = model.action(method).await {
                    tracing::debug!(%error,"Media action was rejected");
                }
            });
        });
    }
    let last_art = std::cell::RefCell::new(String::new());
    let weak = content.downgrade();
    let update = move |model: &MprisModel| {
        if let Some(content) = weak.upgrade() {
            content.set_visible(model.available());
            let uri = model.art_url();
            if *last_art.borrow() != uri {
                last_art.replace(uri.clone());
                while let Some(child) = art.first_child() {
                    art.remove(&child);
                }
                let local = uri.starts_with("file:") || std::path::Path::new(&uri).is_absolute();
                art.set_visible(local);
                placeholder.set_visible(!local);
                if local {
                    let image = gtk::Image::new();
                    image.set_pixel_size(56);
                    crate::notifications::load_local_image(&image, &uri);
                    art.append(&image);
                }
            }
            source.set_text(&model.identity());
            source.set_visible(!model.identity().is_empty());
            title.set_text(&model.title());
            title.set_visible(!model.title().is_empty());
            artist.set_text(&model.artist());
            artist.set_visible(!model.artist().is_empty());
            toggle.set_icon_name(if model.playing() {
                "media-playback-pause-symbolic"
            } else {
                "media-playback-start-symbolic"
            });
            toggle.set_sensitive(model.can_toggle() && !model.busy());
            next.set_sensitive(model.can_next() && !model.busy());
            previous.set_sensitive(model.can_previous() && !model.busy());
        }
    };
    update(model);
    model.connect_changed(update);
    content
}
