//! Reusable keyboard-owning transient surface. Hidden surfaces release the seat.
//!
//! Popups open next to the panel at its start or end. The sidebar is a full
//! height column on the trailing side of the screen that slides in and out.
use asena_core::layout::{PanelEdge, PanelLayout};
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placement {
    Start,
    End,
    Sidebar,
}

#[derive(Clone)]
pub struct Surface {
    pub window: gtk::ApplicationWindow,
    pub content: gtk::Box,
    revealer: Option<gtk::Revealer>,
}

/// The primary display chosen in Settings, else the first one.
fn primary_monitor(display: &gtk::gdk::Display) -> Option<gtk::gdk::Monitor> {
    let connector = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA)
        .map(|settings| settings.string("panel-monitor").to_string())
        .unwrap_or_default();
    let monitors = display.monitors();
    let all: Vec<gtk::gdk::Monitor> = (0..monitors.n_items())
        .filter_map(|index| monitors.item(index)?.downcast().ok())
        .collect();
    all.iter()
        .find(|monitor| monitor.connector().as_deref() == Some(connector.as_str()))
        .or_else(|| all.first())
        .cloned()
}

fn panel_layout(fallback: PanelLayout) -> PanelLayout {
    asena_core::settings::load(asena_core::settings::SHELL_SCHEMA)
        .ok()
        .as_ref()
        .map(PanelLayout::from_settings)
        .unwrap_or(fallback)
}

impl Surface {
    pub fn new(app: &adw::Application, name: &str, layout: PanelLayout, trailing: bool) -> Self {
        let placement = if trailing {
            Placement::End
        } else {
            Placement::Start
        };
        Self::build(app, name, layout, placement)
    }

    /// Full height side panel on the trailing edge of the screen.
    pub fn sidebar(app: &adw::Application, name: &str) -> Self {
        Self::build(app, name, PanelLayout::default(), Placement::Sidebar)
    }

    fn build(
        app: &adw::Application,
        name: &str,
        layout: PanelLayout,
        placement: Placement,
    ) -> Self {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .build();
        window.init_layer_shell();
        window.set_namespace(Some(name));
        window.set_layer(Layer::Top);
        window.set_exclusive_zone(-1);
        window.set_keyboard_mode(KeyboardMode::None);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
        }
        window.add_css_class("asena-scrim");
        let overlay = gtk::Overlay::new();
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.set_hexpand(true);
        backdrop.set_vexpand(true);
        overlay.set_child(Some(&backdrop));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.add_css_class("asena-surface");
        let revealer = (placement == Placement::Sidebar).then(|| {
            let revealer = gtk::Revealer::new();
            revealer.set_transition_duration(200);
            revealer.set_child(Some(&content));
            content.add_css_class("asena-sidebar-surface");
            content.set_vexpand(true);
            revealer
        });
        let positioned: gtk::Widget = match &revealer {
            Some(revealer) => revealer.clone().upcast(),
            None => content.clone().upcast(),
        };
        position(&positioned, panel_layout(layout), placement);
        if let Some(revealer) = &revealer {
            let weak = window.downgrade();
            revealer.connect_child_revealed_notify(move |revealer| {
                if !revealer.is_child_revealed() && !revealer.reveals_child() {
                    if let Some(window) = weak.upgrade() {
                        window.set_keyboard_mode(KeyboardMode::None);
                        window.set_visible(false);
                    }
                }
            });
        }
        if let Ok(preferences) = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA) {
            let weak_target = positioned.downgrade();
            let weak_window = window.downgrade();
            let weak_revealer = revealer.as_ref().map(|revealer| revealer.downgrade());
            preferences.connect_changed(None, move |settings, key| {
                if !matches!(key, "panel-edge" | "panel-thickness") {
                    return;
                }
                if let (Some(target), Some(window)) = (weak_target.upgrade(), weak_window.upgrade())
                {
                    let revealer = weak_revealer.as_ref().and_then(|weak| weak.upgrade());
                    close(&window, revealer.as_ref());
                    position(&target, PanelLayout::from_settings(settings), placement);
                }
            });
        }
        overlay.add_overlay(&positioned);
        window.set_child(Some(&overlay));
        let click = gtk::GestureClick::new();
        let weak = window.downgrade();
        let weak_revealer = revealer.as_ref().map(|revealer| revealer.downgrade());
        click.connect_released(move |_, _, _, _| {
            if let Some(window) = weak.upgrade() {
                let revealer = weak_revealer.as_ref().and_then(|weak| weak.upgrade());
                close(&window, revealer.as_ref());
            }
        });
        backdrop.add_controller(click);
        let keys = gtk::EventControllerKey::new();
        let weak = window.downgrade();
        let weak_revealer = revealer.as_ref().map(|revealer| revealer.downgrade());
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(window) = weak.upgrade() {
                    let revealer = weak_revealer.as_ref().and_then(|weak| weak.upgrade());
                    close(&window, revealer.as_ref());
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(keys);
        Self {
            window,
            content,
            revealer,
        }
    }

    pub fn hide(&self) {
        close(&self.window, self.revealer.as_ref());
    }

    /// Shows or hides the surface; returns true when it was opened.
    pub fn toggle(&self, width: i32, height: i32) -> bool {
        let started = std::time::Instant::now();
        let open = is_open(
            self.window.is_visible(),
            self.revealer
                .as_ref()
                .map(|revealer| (revealer.reveals_child(), revealer.is_child_revealed())),
        );
        if open {
            self.hide();
            return false;
        }
        if let Some(display) = gtk::gdk::Display::default() {
            if let Some(monitor) = primary_monitor(&display) {
                let (geometry, _) = crate::outputs::geometry(&monitor);
                let target_w = width.min(geometry.width() - 24).max(1);
                let target_h = if height > 0 {
                    height.min(geometry.height() - 80).max(1)
                } else {
                    -1
                };
                self.content.set_size_request(target_w, target_h);
                self.window.set_monitor(Some(&monitor));
            }
        }
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.window.present();
        if let Some(revealer) = &self.revealer {
            // Reveal after the first frame so the slide is visible, unless the
            // surface was closed again before that.
            let revealer = revealer.downgrade();
            let window = self.window.downgrade();
            glib::idle_add_local_once(move || {
                if let (Some(revealer), Some(window)) = (revealer.upgrade(), window.upgrade()) {
                    if window.is_visible() {
                        revealer.set_reveal_child(true);
                    }
                }
            });
        }
        if let Some(clock) = self.window.frame_clock() {
            let handler = std::rc::Rc::new(std::cell::RefCell::new(None));
            let pending = handler.clone();
            let id = clock.connect_after_paint(move |clock| {
                tracing::debug!(
                    elapsed_us = started.elapsed().as_micros(),
                    "Transient surface first painted frame"
                );
                if let Some(id) = pending.borrow_mut().take() {
                    clock.disconnect(id);
                }
            });
            handler.replace(Some(id));
            let weak = clock.downgrade();
            glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                if let (Some(clock), Some(id)) = (weak.upgrade(), handler.borrow_mut().take()) {
                    clock.disconnect(id);
                }
            });
        }
        true
    }
}

/// A visible surface is open unless its revealer is sliding out. The reveal
/// starts after the first frame, so a surface that was just presented counts
/// as open before its revealer moves; a toggle that arrives then closes it.
/// `revealer` holds `(reveals_child, child_revealed)`.
fn is_open(visible: bool, revealer: Option<(bool, bool)>) -> bool {
    visible && revealer.is_none_or(|(reveals, revealed)| reveals || !revealed)
}

fn close(window: &gtk::ApplicationWindow, revealer: Option<&gtk::Revealer>) {
    match revealer {
        // The window hides once the slide out finishes.
        Some(revealer) if revealer.is_child_revealed() && window.is_visible() => {
            revealer.set_reveal_child(false);
        }
        _ => {
            if let Some(revealer) = revealer {
                revealer.set_reveal_child(false);
            }
            window.set_keyboard_mode(KeyboardMode::None);
            window.set_visible(false);
        }
    }
}

fn position(target: &gtk::Widget, layout: PanelLayout, placement: Placement) {
    target.set_margin_top(8);
    target.set_margin_bottom(8);
    target.set_margin_start(8);
    target.set_margin_end(8);
    let gap = layout.thickness + 8;
    if placement == Placement::Sidebar {
        // Trailing side; next to the panel when the panel is on the right.
        target.set_halign(gtk::Align::End);
        target.set_valign(gtk::Align::Fill);
        match layout.edge {
            PanelEdge::Bottom => target.set_margin_bottom(gap),
            PanelEdge::Top => target.set_margin_top(gap),
            PanelEdge::Right => target.set_margin_end(gap),
            PanelEdge::Left => {}
        }
        if let Some(revealer) = target.downcast_ref::<gtk::Revealer>() {
            revealer.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
        }
        return;
    }
    let trailing = placement == Placement::End;
    match layout.edge {
        PanelEdge::Bottom | PanelEdge::Top => {
            target.set_halign(if trailing {
                gtk::Align::End
            } else {
                gtk::Align::Start
            });
            target.set_valign(if layout.edge == PanelEdge::Bottom {
                gtk::Align::End
            } else {
                gtk::Align::Start
            });
            if layout.edge == PanelEdge::Bottom {
                target.set_margin_bottom(gap);
            } else {
                target.set_margin_top(gap);
            }
        }
        PanelEdge::Left | PanelEdge::Right => {
            target.set_halign(if layout.edge == PanelEdge::Left {
                gtk::Align::Start
            } else {
                gtk::Align::End
            });
            target.set_valign(if trailing {
                gtk::Align::End
            } else {
                gtk::Align::Start
            });
            if layout.edge == PanelEdge::Left {
                target.set_margin_start(gap);
            } else {
                target.set_margin_end(gap);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_open;

    #[test]
    fn hidden_surfaces_are_closed() {
        assert!(!is_open(false, None));
        assert!(!is_open(false, Some((true, true))));
    }

    #[test]
    fn popups_are_open_while_visible() {
        assert!(is_open(true, None));
    }

    #[test]
    fn sidebar_is_open_before_its_reveal_starts() {
        // Presented, reveal still waiting for the first frame.
        assert!(is_open(true, Some((false, false))));
        assert!(is_open(true, Some((true, false))));
        assert!(is_open(true, Some((true, true))));
    }

    #[test]
    fn sliding_out_sidebar_reopens_on_toggle() {
        assert!(!is_open(true, Some((false, true))));
    }
}
