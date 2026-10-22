use asena_core::{
    i18n::{gettext, ngettext},
    taskbar::{groups, Group},
};
use asena_services::{app_catalog::AppCatalog, toplevel_model::ToplevelModel};
use asena_wayland::toplevel::Request;
use gtk::prelude::*;
use std::cell::RefCell;

fn current_groups(
    model: &ToplevelModel,
    catalog: &AppCatalog,
    settings: &gio::Settings,
) -> Vec<Group> {
    let windows = model.windows();
    let ids: Vec<_> = windows
        .iter()
        .map(|window| (window.id(), window.app_id()))
        .collect();
    let pins = settings
        .strv("pinned-apps")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    catalog.with_index(|index| {
        groups(
            index,
            &pins,
            ids.iter().map(|(id, app_id)| (*id, app_id.as_str())),
        )
    })
}

fn popup(button: &gtk::Button, content: &gtk::Box, position: gtk::PositionType) {
    use gtk4_layer_shell::{KeyboardMode, LayerShell};
    let window = button.root().and_downcast::<gtk::ApplicationWindow>();
    if let Some(window) = &window {
        window.set_keyboard_mode(KeyboardMode::OnDemand);
    }
    let popover = gtk::Popover::new();
    popover.set_position(position);
    popover.set_has_arrow(false);
    popover.set_child(Some(content));
    popover.set_parent(button);
    let weak_window = window.as_ref().map(|window| window.downgrade());
    popover.connect_closed(move |popover| {
        if let Some(window) = weak_window.as_ref().and_then(glib::WeakRef::upgrade) {
            window.set_keyboard_mode(KeyboardMode::None);
        }
        if popover.parent().is_some() {
            popover.unparent();
        }
    });
    let weak_popover = popover.downgrade();
    button.connect_unrealize(move |_| {
        if let Some(popover) = weak_popover.upgrade() {
            popover.popdown();
            if popover.parent().is_some() {
                popover.unparent();
            }
        }
    });
    popover.popup();
}

fn window_list(
    button: &gtk::Button,
    group: &Group,
    model: &ToplevelModel,
    position: gtk::PositionType,
) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for window in model
        .windows()
        .into_iter()
        .filter(|window| group.windows.contains(&window.id()))
    {
        let title = if window.title().is_empty() {
            gettext("Untitled window")
        } else {
            window.title()
        };
        let item = gtk::Button::with_label(&title);
        item.add_css_class("flat");
        let model = model.clone();
        item.connect_clicked(move |item| {
            model.request(Request::Activate(window.id()));
            if let Some(popover) = item
                .ancestor(gtk::Popover::static_type())
                .and_downcast::<gtk::Popover>()
            {
                popover.popdown();
            }
        });
        content.append(&item);
    }
    popup(button, &content, position);
}

fn context_menu(
    button: &gtk::Button,
    group: &Group,
    model: &ToplevelModel,
    settings: &gio::Settings,
    position: gtk::PositionType,
) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let pin = gtk::Button::with_label(&if group.pinned {
        gettext("Unpin from taskbar")
    } else {
        gettext("Pin to taskbar")
    });
    pin.add_css_class("flat");
    pin.set_sensitive(group.app.is_some() || group.pinned);
    let key = group
        .app
        .as_ref()
        .map(|app| app.id.clone())
        .unwrap_or_else(|| group.app_id.clone());
    let pinned = group.pinned;
    let settings = settings.clone();
    pin.connect_clicked(move |item| {
        let mut pins = settings
            .strv("pinned-apps")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if pinned {
            pins.retain(|pin| pin != &key);
        } else if !pins.contains(&key) {
            pins.push(key.clone());
        }
        if let Err(error) = settings.set_strv("pinned-apps", pins) {
            tracing::warn!(%error, "Unable to save taskbar pins");
        }
        if let Some(popover) = item
            .ancestor(gtk::Popover::static_type())
            .and_downcast::<gtk::Popover>()
        {
            popover.popdown();
        }
    });
    content.append(&pin);
    let new = gtk::Button::with_label(&gettext("New window"));
    new.add_css_class("flat");
    new.set_sensitive(group.app.is_some());
    let app = group.app.clone();
    new.connect_clicked(move |item| {
        if let Some(app) = app.clone() {
            crate::launch::launch(app, item.upcast_ref());
        }
    });
    content.append(&new);
    if !group.windows.is_empty() {
        let close = gtk::Button::with_label(&gettext("Close all windows"));
        close.add_css_class("flat");
        let ids = group.windows.clone();
        let model = model.clone();
        close.connect_clicked(move |item| {
            if ids.len() == 1 {
                model.request(Request::Close(ids[0]));
                return;
            }
            let dialog = adw::AlertDialog::builder()
                .heading(gettext("Close all windows?"))
                .build();
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("close-all", &gettext("Close all"));
            dialog.set_response_appearance("close-all", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            let model = model.clone();
            let ids = ids.clone();
            dialog.connect_response(Some("close-all"), move |_, _| {
                for id in &ids {
                    model.request(Request::Close(*id));
                }
            });
            use adw::prelude::*;
            dialog.present(Some(item));
        });
        content.append(&close);
    }
    popup(button, &content, position);
}

fn render(
    root: &gtk::Box,
    model: &ToplevelModel,
    catalog: &AppCatalog,
    settings: &gio::Settings,
    position: gtk::PositionType,
) {
    // Views are derived from the model; action handlers look up current state.
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
    for group in current_groups(model, catalog, settings) {
        let button = gtk::Button::new();
        button.add_css_class("flat");
        button.add_css_class("asena-task");
        // The icon stays centered whether or not the application runs; the
        // indicator is layered on the screen edge side of the button.
        let content = gtk::Overlay::new();
        let icon = group
            .app
            .as_ref()
            .and_then(|app| app.icon.as_ref())
            .and_then(|icon| gio::Icon::for_string(icon).ok());
        let image = if let Some(icon) = icon {
            gtk::Image::from_gicon(&icon)
        } else {
            gtk::Image::from_icon_name("org.asenalinux.App")
        };
        image.set_pixel_size(
            settings
                .int("icon-size")
                .min(settings.int("panel-thickness") - 12),
        );
        image.set_halign(gtk::Align::Center);
        image.set_valign(gtk::Align::Center);
        content.set_child(Some(&image));
        let vertical_panel = matches!(position, gtk::PositionType::Left | gtk::PositionType::Right);
        let indicators = gtk::Box::new(
            if vertical_panel {
                gtk::Orientation::Vertical
            } else {
                gtk::Orientation::Horizontal
            },
            3,
        );
        indicators.add_css_class("asena-task-indicators");
        indicators.set_can_target(false);
        let (halign, valign) = match position {
            // `position` faces into the screen; the indicator sits on the screen edge.
            gtk::PositionType::Top => (gtk::Align::Center, gtk::Align::End),
            gtk::PositionType::Bottom => (gtk::Align::Center, gtk::Align::Start),
            gtk::PositionType::Right => (gtk::Align::Start, gtk::Align::Center),
            _ => (gtk::Align::End, gtk::Align::Center),
        };
        indicators.set_halign(halign);
        indicators.set_valign(valign);
        for _ in 0..group.windows.len().min(2) {
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("asena-dot");
            if vertical_panel {
                dot.add_css_class("vertical");
            }
            indicators.append(&dot);
        }
        content.add_overlay(&indicators);
        button.set_child(Some(&content));
        if model
            .windows()
            .iter()
            .any(|window| group.windows.contains(&window.id()) && window.activated())
        {
            button.add_css_class("active");
        }
        let name = group
            .app
            .as_ref()
            .map(|app| app.name.clone())
            .unwrap_or_else(|| {
                if group.app_id.is_empty() {
                    gettext("Application")
                } else {
                    group.app_id.clone()
                }
            });
        let tip = ngettext(
            "{name}, {count} window",
            "{name}, {count} windows",
            group.windows.len() as u32,
        )
        .replace("{name}", &name)
        .replace("{count}", &group.windows.len().to_string());
        button.set_tooltip_text(Some(&tip));
        button.update_property(&[gtk::accessible::Property::Label(&name)]);
        let key = group.key.clone();
        let model_click = model.clone();
        let catalog_click = catalog.clone();
        let settings_click = settings.clone();
        button.connect_clicked(move |button| {
            let Some(group) = current_groups(&model_click, &catalog_click, &settings_click)
                .into_iter()
                .find(|group| group.key == key)
            else {
                return;
            };
            match group.windows.as_slice() {
                [] => {
                    if let Some(app) = group.app {
                        crate::launch::launch(app, button.upcast_ref());
                    }
                }
                [id] => {
                    let active = model_click
                        .windows()
                        .iter()
                        .any(|window| window.id() == *id && window.activated());
                    model_click.request(if active {
                        Request::Minimize(*id)
                    } else {
                        Request::Activate(*id)
                    });
                }
                _ => window_list(button, &group, &model_click, position),
            }
        });
        let gesture = gtk::GestureClick::new();
        gesture.set_button(0);
        let weak = button.downgrade();
        let model = model.clone();
        let settings = settings.clone();
        gesture.connect_pressed(move |gesture, _, _, _| {
            let Some(button) = weak.upgrade() else {
                return;
            };
            match gesture.current_button() {
                2 => {
                    if let Some(app) = group.app.clone() {
                        crate::launch::launch(app, button.upcast_ref());
                    }
                }
                3 => context_menu(&button, &group, &model, &settings, position),
                _ => return,
            }
            gesture.set_state(gtk::EventSequenceState::Claimed);
        });
        button.add_controller(gesture);
        root.append(&button);
    }
}

pub fn create(
    model: &ToplevelModel,
    catalog: &AppCatalog,
    settings: &gio::Settings,
    layout: asena_core::layout::PanelLayout,
) -> crate::taskbar_strip::TaskbarStrip {
    use asena_core::layout::PanelEdge;
    let position = match layout.inward_edge() {
        PanelEdge::Top => gtk::PositionType::Top,
        PanelEdge::Bottom => gtk::PositionType::Bottom,
        PanelEdge::Left => gtk::PositionType::Left,
        PanelEdge::Right => gtk::PositionType::Right,
    };
    let axis = if layout.is_horizontal() {
        gtk::Orientation::Horizontal
    } else {
        gtk::Orientation::Vertical
    };
    let root = gtk::Box::new(axis, 4);
    let align = match settings.string("panel-alignment").as_str() {
        "center" => gtk::Align::Center,
        "end" => gtk::Align::End,
        _ => gtk::Align::Start,
    };
    if layout.is_horizontal() {
        root.set_halign(align);
    } else {
        root.set_valign(align);
    }
    render(&root, model, catalog, settings, position);
    let weak = root.downgrade();
    let apps = catalog.clone();
    let prefs = settings.clone();
    let model_handler = model.connect_changed(move |model| {
        if let Some(root) = weak.upgrade() {
            render(&root, model, &apps, &prefs, position);
        }
    });
    let weak = root.downgrade();
    let windows = model.clone();
    let prefs = settings.clone();
    let catalog_handler = catalog.connect_changed(move |catalog| {
        if let Some(root) = weak.upgrade() {
            render(&root, &windows, catalog, &prefs, position);
        }
    });
    let weak = root.downgrade();
    let windows = model.clone();
    let apps = catalog.clone();
    let settings_handler = settings.connect_changed(Some("pinned-apps"), move |settings, _| {
        if let Some(root) = weak.upgrade() {
            render(&root, &windows, &apps, settings, position);
        }
    });
    let handlers = RefCell::new(Some((model_handler, catalog_handler, settings_handler)));
    let cleanup_model = model.clone();
    let cleanup_catalog = catalog.clone();
    let cleanup_settings = settings.clone();
    root.connect_unrealize(move |_| {
        if let Some((a, b, c)) = handlers.borrow_mut().take() {
            cleanup_model.disconnect(a);
            cleanup_catalog.disconnect(b);
            cleanup_settings.disconnect(c);
        }
    });
    let scroll = gtk::ScrolledWindow::builder()
        .child(&root)
        .hexpand(layout.is_horizontal())
        .vexpand(!layout.is_horizontal())
        .hscrollbar_policy(if layout.is_horizontal() {
            gtk::PolicyType::External
        } else {
            gtk::PolicyType::Never
        })
        .vscrollbar_policy(if layout.is_horizontal() {
            gtk::PolicyType::Never
        } else {
            gtk::PolicyType::External
        })
        .min_content_width(0)
        .build();
    let more = gtk::MenuButton::new();
    more.set_icon_name("view-more-symbolic");
    more.set_tooltip_text(Some(&gettext("More applications")));
    more.update_property(&[gtk::accessible::Property::Label(&gettext(
        "More applications",
    ))]);
    more.add_css_class("flat");
    more.set_visible(false);
    let windows = model.clone();
    let apps = catalog.clone();
    let prefs = settings.clone();
    more.set_create_popup_func(move |more| {
        let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
        for group in current_groups(&windows, &apps, &prefs) {
            if group.windows.is_empty() {
                if let Some(app) = group.app {
                    let item = gtk::Button::with_label(&app.name);
                    item.add_css_class("flat");
                    item.connect_clicked(move |item| {
                        crate::launch::launch(app.clone(), item.upcast_ref())
                    });
                    content.append(&item);
                }
            } else {
                for window in windows
                    .windows()
                    .into_iter()
                    .filter(|window| group.windows.contains(&window.id()))
                {
                    let title = if window.title().is_empty() {
                        gettext("Untitled window")
                    } else {
                        window.title()
                    };
                    let item = gtk::Button::with_label(&title);
                    item.add_css_class("flat");
                    if let Some(label) = item.child().and_downcast::<gtk::Label>() {
                        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                        label.set_max_width_chars(40);
                    }
                    let model = windows.clone();
                    item.connect_clicked(move |item| {
                        model.request(Request::Activate(window.id()));
                        if let Some(popover) = item
                            .ancestor(gtk::Popover::static_type())
                            .and_downcast::<gtk::Popover>()
                        {
                            popover.popdown();
                        }
                    });
                    content.append(&item);
                }
            }
        }
        let scroll = gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .max_content_height(480)
            .propagate_natural_height(true)
            .build();
        let popover = gtk::Popover::new();
        popover.set_position(position);
        popover.set_has_arrow(false);
        popover.set_child(Some(&scroll));
        more.set_popover(Some(&popover));
    });
    more.connect_active_notify(|button| {
        use gtk4_layer_shell::{KeyboardMode, LayerShell};
        if let Some(window) = button.root().and_downcast::<gtk::ApplicationWindow>() {
            window.set_keyboard_mode(if button.is_active() {
                KeyboardMode::OnDemand
            } else {
                KeyboardMode::None
            });
        }
    });
    crate::taskbar_strip::TaskbarStrip::new(&root, &scroll, &more)
}
