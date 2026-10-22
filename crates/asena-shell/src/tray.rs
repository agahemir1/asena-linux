use asena_core::{
    dbusmenu::Node,
    layout::{PanelEdge, PanelLayout},
};
use asena_services::tray::{Item, TrayModel};
use gtk::prelude::*;
use gtk4_layer_shell::{KeyboardMode, LayerShell};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};

pub fn create(
    model: &TrayModel,
    parent: &gtk::ApplicationWindow,
    monitor: &gtk::gdk::Monitor,
    layout: PanelLayout,
) -> gtk::Box {
    let content = gtk::Box::new(
        if layout.is_horizontal() {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        },
        0,
    );
    let rows = Rc::new(RefCell::new(
        BTreeMap::<String, (gtk::Button, gtk::Image)>::new(),
    ));
    let weak = content.downgrade();
    let parent = parent.downgrade();
    let monitor = monitor.clone();
    let update = move |model: &TrayModel| {
        let Some(content) = weak.upgrade() else {
            return;
        };
        let items = model.items();
        rows.borrow_mut().retain(|id, (button, _)| {
            if items.iter().any(|item| &item.id == id) {
                true
            } else {
                content.remove(button);
                false
            }
        });
        for item in items {
            if !rows.borrow().contains_key(&item.id) {
                let button = gtk::Button::new();
                button.add_css_class("flat");
                let image = gtk::Image::new();
                image.set_pixel_size(24);
                button.set_child(Some(&image));
                let active = model.clone();
                let id = item.id.clone();
                let window = parent.clone();
                let output = monitor.clone();
                button.connect_clicked(move |button| {
                    let (x, y) = coordinates(button, window.upgrade().as_ref(), &output, layout);
                    let menu = active
                        .items()
                        .iter()
                        .find(|item| item.id == id)
                        .is_some_and(|item| item.properties.boolean("ItemIsMenu") == Some(true));
                    if menu {
                        open_menu(button, &active, &id, window.clone(), x, y);
                    } else {
                        invoke(&active, &id, "Activate", x, y);
                    }
                });
                for (number, method) in [(2, "SecondaryActivate"), (3, "ContextMenu")] {
                    let click = gtk::GestureClick::new();
                    click.set_button(number);
                    let active = model.clone();
                    let id = item.id.clone();
                    let button_weak = button.downgrade();
                    let window = parent.clone();
                    let output = monitor.clone();
                    click.connect_released(move |_, _, _, _| {
                        if let Some(button) = button_weak.upgrade() {
                            let (x, y) =
                                coordinates(&button, window.upgrade().as_ref(), &output, layout);
                            if method == "ContextMenu" {
                                open_menu(&button, &active, &id, window.clone(), x, y);
                            } else {
                                invoke(&active, &id, method, x, y);
                            }
                        }
                    });
                    button.add_controller(click);
                }
                content.append(&button);
                rows.borrow_mut().insert(item.id.clone(), (button, image));
            }
            if let Some((button, image)) = rows.borrow().get(&item.id) {
                update_icon(image, &item);
                button.set_tooltip_text(Some(&tooltip(&item)));
            }
        }
    };
    update(model);
    let handler = model.connect_changed(update);
    let model = model.clone();
    let handler = RefCell::new(Some(handler));
    content.connect_unrealize(move |_| {
        if let Some(handler) = handler.borrow_mut().take() {
            model.disconnect(handler);
        }
    });
    content
}
fn coordinates(
    button: &gtk::Button,
    parent: Option<&gtk::ApplicationWindow>,
    monitor: &gtk::gdk::Monitor,
    layout: PanelLayout,
) -> (i32, i32) {
    let (geometry, _) = crate::outputs::geometry(monitor);
    let point = parent
        .and_then(|parent| button.compute_point(parent, &gtk::graphene::Point::new(0.0, 0.0)))
        .unwrap_or(gtk::graphene::Point::new(0.0, 0.0));
    let x = geometry.x()
        + if layout.edge == PanelEdge::Right {
            geometry.width() - layout.thickness
        } else {
            0
        };
    let y = geometry.y()
        + if layout.edge == PanelEdge::Bottom {
            geometry.height() - layout.thickness
        } else {
            0
        };
    (x + point.x() as i32, y + point.y() as i32)
}
fn tooltip(item: &Item) -> String {
    item.properties
        .with_values(|values| {
            let tip = values
                .get("ToolTip")
                .and_then(|v| v.try_clone().ok())
                .and_then(|v| {
                    <(String, Vec<(i32, i32, Vec<u8>)>, String, String)>::try_from(v).ok()
                });
            if let Some((_, _, title, body)) = tip {
                if !title.is_empty() {
                    return if body.is_empty() {
                        title
                    } else {
                        format!("{title}\n{body}")
                    };
                }
            }
            item.properties.string("Title").unwrap_or_default()
        })
        .chars()
        .take(4096)
        .collect()
}
fn update_icon(image: &gtk::Image, item: &Item) {
    let attention = item.properties.string("Status").as_deref() == Some("NeedsAttention");
    let name = item
        .properties
        .string(if attention {
            "AttentionIconName"
        } else {
            "IconName"
        })
        .filter(|name| !name.is_empty());
    if let Some(name) = name {
        image.set_icon_name(Some(&name));
        return;
    }
    let pixels = item.properties.with_values(|values| {
        values
            .get(if attention {
                "AttentionIconPixmap"
            } else {
                "IconPixmap"
            })
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| Vec::<(i32, i32, Vec<u8>)>::try_from(v).ok())
            .and_then(|frames| asena_core::tray::pixmap(&frames, 24 * image.scale_factor()))
    });
    if let Some(pixels) = pixels {
        let bytes = glib::Bytes::from_owned(pixels.rgba);
        let texture = gtk::gdk::MemoryTexture::new(
            pixels.width,
            pixels.height,
            gtk::gdk::MemoryFormat::R8g8b8a8,
            &bytes,
            pixels.width as usize * 4,
        );
        image.set_paintable(Some(&texture));
    } else {
        image.set_icon_name(Some("application-x-executable-symbolic"));
    }
}
fn invoke(model: &TrayModel, id: &str, method: &'static str, x: i32, y: i32) {
    let model = model.clone();
    let id = id.to_owned();
    glib::spawn_future_local(async move {
        if let Err(error) = model.invoke(&id, method, x, y).await {
            tracing::debug!(%error,"Tray item action failed");
        }
    });
}
fn open_menu(
    button: &gtk::Button,
    model: &TrayModel,
    id: &str,
    parent: glib::WeakRef<gtk::ApplicationWindow>,
    x: i32,
    y: i32,
) {
    if model.menu_path(id).is_none() {
        invoke(model, id, "ContextMenu", x, y);
        return;
    }
    let button = button.downgrade();
    let model = model.clone();
    let id = id.to_owned();
    glib::spawn_future_local(async move {
        let (updates, receiver) = async_channel::bounded(1);
        let active = model.clone();
        let target = id.clone();
        let watch = glib::spawn_future_local(async move {
            if let Err(error) = active.watch_menu(&target, updates).await {
                tracing::debug!(%error,"Tray menu stream stopped");
            }
        });
        let initial = futures_lite::future::race(async { receiver.recv().await.ok() }, async {
            glib::timeout_future(Duration::from_secs(3)).await;
            None
        })
        .await;
        let (Some(node), Some(button), Some(parent)) =
            (initial, button.upgrade(), parent.upgrade())
        else {
            watch.abort();
            invoke(&model, &id, "ContextMenu", x, y);
            return;
        };
        let popover = gtk::Popover::new();
        popover.set_parent(&button);
        popover.set_autohide(true);
        popover.set_child(Some(&menu_box(&node, &model, &id, &popover)));
        parent.set_keyboard_mode(KeyboardMode::OnDemand);
        let weak = popover.downgrade();
        let active = model.clone();
        let target = id.clone();
        let updater = glib::spawn_future_local(async move {
            while let Ok(node) = receiver.recv().await {
                let Some(popover) = weak.upgrade() else {
                    break;
                };
                popover.set_child(Some(&menu_box(&node, &active, &target, &popover)));
            }
        });
        let tasks = Rc::new(RefCell::new(Some((watch, updater))));
        let pending = tasks.clone();
        let window = parent.downgrade();
        popover.connect_closed(move |popover| {
            if let Some((watch, updater)) = pending.borrow_mut().take() {
                watch.abort();
                updater.abort();
            }
            if let Some(window) = window.upgrade() {
                window.set_keyboard_mode(KeyboardMode::None);
            }
            popover.unparent();
        });
        let window = parent.downgrade();
        popover.connect_unrealize(move |_| {
            if let Some((watch, updater)) = tasks.borrow_mut().take() {
                watch.abort();
                updater.abort();
            }
            if let Some(window) = window.upgrade() {
                window.set_keyboard_mode(KeyboardMode::None);
            }
        });
        popover.popup();
    });
}
fn menu_box(node: &Node, model: &TrayModel, id: &str, root: &gtk::Popover) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for node in &node.children {
        if !node.visible {
            continue;
        }
        if node.separator {
            content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            continue;
        }
        if !node.children.is_empty() {
            let submenu = gtk::MenuButton::new();
            submenu.set_label(&node.label);
            submenu.set_sensitive(node.enabled);
            let popover = gtk::Popover::new();
            popover.set_child(Some(&menu_box(node, model, id, root)));
            submenu.set_popover(Some(&popover));
            content.append(&submenu);
        } else {
            let button = gtk::Button::with_mnemonic(&node.label);
            button.add_css_class("flat");
            button.set_sensitive(node.enabled);
            if let Some(checked) = node.checked {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let indicator = gtk::Image::from_icon_name("object-select-symbolic");
                indicator.set_opacity(if checked { 1.0 } else { 0.0 });
                let label = gtk::Label::with_mnemonic(&node.label);
                label.set_mnemonic_widget(Some(&button));
                label.set_xalign(0.0);
                row.append(&indicator);
                row.append(&label);
                button.set_child(Some(&row));
            }
            let model = model.clone();
            let id = id.to_owned();
            let key = node.id;
            let root = root.downgrade();
            button.connect_clicked(move |_| {
                if let Some(root) = root.upgrade() {
                    root.popdown();
                }
                let model = model.clone();
                let id = id.clone();
                glib::spawn_future_local(async move {
                    if let Err(error) = model.menu_event(&id, key, 0).await {
                        tracing::debug!(%error,"Tray menu event failed");
                    }
                });
            });
            content.append(&button);
        }
    }
    content
}
