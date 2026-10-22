//! Panel status cluster: network, Bluetooth, volume, battery and do not disturb.
//!
//! Each indicator follows its service model and hides itself when the service
//! or the hardware is missing. Clicking the cluster opens quick settings.
use asena_core::{i18n::gettext, status};
use asena_services::system::{self, SystemServices};
use gtk::prelude::*;
use std::rc::Rc;

struct Indicators {
    network: gtk::Image,
    bluetooth: gtk::Image,
    volume: gtk::Image,
    battery: gtk::Image,
    battery_label: gtk::Label,
    quiet: gtk::Image,
    button: gtk::Button,
}

fn set(image: &gtk::Image, icon: Option<&str>) {
    image.set_visible(icon.is_some());
    if let Some(icon) = icon {
        image.set_icon_name(Some(icon));
    }
}

impl Indicators {
    fn refresh(&self, services: &SystemServices, notifications: &gio::Settings) {
        let mut summary = Vec::new();
        let (network, ssid) = system::network_state(&services.network, &services.network_objects);
        set(&self.network, status::network_icon(&network));
        if network.available {
            summary.push(match (network.primary_type.as_str(), ssid) {
                ("802-11-wireless", Some(name)) => {
                    gettext("Wi-Fi: {name}").replace("{name}", &name)
                }
                ("802-3-ethernet", _) => gettext("Wired connection"),
                _ if network.state >= status::NM_STATE_CONNECTED_LOCAL => gettext("Connected"),
                _ => gettext("Not connected"),
            });
        }

        let adapters = asena_services::bluetooth::adapters(&services.bluetooth);
        let powered = adapters.iter().any(|adapter| adapter.powered);
        let connected = adapters
            .iter()
            .flat_map(|adapter| {
                asena_services::bluetooth::devices(&services.bluetooth, &adapter.path)
            })
            .filter(|device| device.connected)
            .count();
        set(
            &self.bluetooth,
            status::bluetooth_icon(
                services.bluetooth.available() && !adapters.is_empty(),
                powered,
                connected,
            ),
        );

        let audio = &services.audio;
        let audio_ready = audio.available() && audio.output_available();
        set(
            &self.volume,
            status::volume_icon(audio_ready, audio.volume(), audio.muted()),
        );
        if audio_ready {
            summary.push(if audio.muted() {
                gettext("Sound muted")
            } else {
                gettext("Volume {percent}%")
                    .replace("{percent}", &format!("{:.0}", audio.volume() * 100.0))
            });
        }

        let percentage = system::battery_percentage(&services.battery);
        let battery = status::battery_icon(percentage, system::battery_state(&services.battery));
        set(&self.battery, battery.as_deref());
        self.battery_label.set_visible(false);
        if let Some(value) = percentage {
            let text = format!("{value:.0}%");
            // Show the number only when it matters for a decision.
            self.battery_label.set_text(&text);
            self.battery_label.set_visible(value <= 20.0);
            summary.push(gettext("Battery {percent}").replace("{percent}", &text));
        }

        let quiet = notifications.boolean("do-not-disturb");
        set(
            &self.quiet,
            quiet.then_some("notifications-disabled-symbolic"),
        );
        if quiet {
            summary.push(gettext("Do not disturb"));
        }

        let label = gettext("System status");
        let tooltip = if summary.is_empty() {
            label.clone()
        } else {
            summary.join("\n")
        };
        self.button.set_tooltip_text(Some(&tooltip));
        self.button
            .update_property(&[gtk::accessible::Property::Description(&tooltip)]);
    }
}

pub fn create(
    services: &SystemServices,
    notifications: &gio::Settings,
    orientation: gtk::Orientation,
    on_click: impl Fn() + 'static,
) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("flat");
    button.add_css_class("asena-status");
    let label = gettext("System status");
    button.update_property(&[gtk::accessible::Property::Label(&label)]);
    let content = gtk::Box::new(orientation, 6);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    let image = || {
        let image = gtk::Image::new();
        image.set_visible(false);
        image.add_css_class("asena-status-icon");
        image
    };
    let indicators = Rc::new(Indicators {
        network: image(),
        bluetooth: image(),
        volume: image(),
        battery: image(),
        battery_label: gtk::Label::new(None),
        quiet: image(),
        button: button.clone(),
    });
    indicators.battery_label.add_css_class("asena-status-label");
    indicators.battery_label.set_visible(false);
    for widget in [
        indicators.quiet.upcast_ref::<gtk::Widget>(),
        indicators.network.upcast_ref(),
        indicators.bluetooth.upcast_ref(),
        indicators.volume.upcast_ref(),
        indicators.battery.upcast_ref(),
        indicators.battery_label.upcast_ref(),
    ] {
        content.append(widget);
    }
    // A fixed glyph keeps the button reachable when no service reports state.
    let fallback = gtk::Image::from_icon_name("emblem-system-symbolic");
    fallback.add_css_class("asena-status-icon");
    content.append(&fallback);
    button.set_child(Some(&content));

    let weak = Rc::downgrade(&indicators);
    let refresh: Rc<dyn Fn()> = {
        let services = services.clone();
        let notifications = notifications.clone();
        let fallback = fallback.downgrade();
        Rc::new(move || {
            let (Some(indicators), Some(fallback)) = (weak.upgrade(), fallback.upgrade()) else {
                return;
            };
            indicators.refresh(&services, &notifications);
            let any = [
                &indicators.network,
                &indicators.bluetooth,
                &indicators.volume,
                &indicators.battery,
                &indicators.quiet,
            ]
            .iter()
            .any(|image| image.is_visible());
            fallback.set_visible(!any);
        })
    };
    refresh();
    let watch = |connect: &dyn Fn(Rc<dyn Fn()>) -> glib::SignalHandlerId| connect(refresh.clone());
    let network = watch(&|f| services.network.connect_changed(move |_| f()));
    let objects = watch(&|f| services.network_objects.connect_changed(move |_| f()));
    let bluetooth = watch(&|f| services.bluetooth.connect_changed(move |_| f()));
    let audio = watch(&|f| services.audio.connect_changed(move |_| f()));
    let battery = watch(&|f| services.battery.connect_changed(move |_| f()));
    let quiet = watch(&|f| notifications.connect_changed(Some("do-not-disturb"), move |_, _| f()));
    let handlers =
        std::cell::RefCell::new(Some((network, objects, bluetooth, audio, battery, quiet)));
    let models = (services.clone(), notifications.clone());
    // The indicators live as long as the button; models outlive panels.
    let owner = indicators.clone();
    button.connect_destroy(move |_| {
        let _ = &owner;
        if let Some((network, objects, bluetooth, audio, battery, quiet)) =
            handlers.borrow_mut().take()
        {
            models.0.network.disconnect(network);
            models.0.network_objects.disconnect(objects);
            models.0.bluetooth.disconnect(bluetooth);
            models.0.audio.disconnect(audio);
            models.0.battery.disconnect(battery);
            models.1.disconnect(quiet);
        }
    });
    button.connect_clicked(move |_| on_click());
    button
}
