//! Output geometry read from the compositor's output heads.
//!
//! GTK 4.18 keeps a wrong geometry and scale for a monitor whose scale goes
//! back to 1: its xdg-output code divides by the integer scale it derived
//! from the previous scale (fixed in GTK 4.20). Shell sizing and placement
//! read the heads instead and fall back to GTK without output management.
use asena_core::displays::{Config, Mode};
use asena_wayland::output::{Event, Head};
use gtk::gdk;
use std::{cell::RefCell, rc::Rc};

type Listener = Rc<dyn Fn()>;

thread_local! {
    static HEADS: RefCell<Vec<Head>> = const { RefCell::new(Vec::new()) };
    static LISTENERS: RefCell<Vec<(u64, Listener)>> = const { RefCell::new(Vec::new()) };
    static NEXT: RefCell<u64> = const { RefCell::new(0) };
}

/// Follows the output heads for the rest of the process. Main thread only.
pub fn start() {
    let bridge = match asena_wayland::output::start() {
        Ok(bridge) => bridge,
        Err(error) => {
            tracing::warn!(%error, "Output heads are unavailable; using GTK monitor geometry");
            return;
        }
    };
    glib::spawn_future_local(async move {
        while let Ok(event) = bridge.events.recv().await {
            match event {
                Event::Heads(heads) => HEADS.with(|current| current.replace(heads)),
                Event::Available(false) => HEADS.with(|current| current.take()),
                Event::Available(true) => continue,
            };
            // Clone first: a listener may connect or disconnect others.
            let listeners: Vec<Listener> = LISTENERS.with(|listeners| {
                listeners
                    .borrow()
                    .iter()
                    .map(|(_, listener)| listener.clone())
                    .collect()
            });
            for listener in listeners {
                listener();
            }
        }
    });
}

/// Runs `listener` after every update of the heads.
pub fn connect_changed(listener: impl Fn() + 'static) -> u64 {
    let id = NEXT.with(|next| {
        let mut next = next.borrow_mut();
        *next += 1;
        *next
    });
    LISTENERS.with(|listeners| listeners.borrow_mut().push((id, Rc::new(listener))));
    id
}

pub fn disconnect(id: u64) {
    LISTENERS.with(|listeners| listeners.borrow_mut().retain(|(known, _)| *known != id));
}

/// Logical position and size, and the fractional scale.
fn rect(head: &Head) -> Option<(gdk::Rectangle, f64)> {
    if !head.enabled {
        return None;
    }
    let config = Config {
        name: head.name.clone(),
        enabled: true,
        mode: head.mode.map(|mode| Mode {
            width: mode.width,
            height: mode.height,
            refresh: mode.refresh,
            preferred: mode.preferred,
        }),
        x: head.x,
        y: head.y,
        transform: head.transform,
        scale: head.scale,
    };
    let rect = config.rect()?;
    Some((
        gdk::Rectangle::new(rect.x, rect.y, rect.width, rect.height),
        head.scale,
    ))
}

/// The monitor's logical geometry and scale.
pub fn geometry(monitor: &gdk::Monitor) -> (gdk::Rectangle, f64) {
    use gtk::prelude::MonitorExt;
    let connector = monitor.connector();
    HEADS
        .with(|heads| {
            heads
                .borrow()
                .iter()
                .find(|head| Some(head.name.as_str()) == connector.as_deref())
                .and_then(rect)
        })
        .unwrap_or_else(|| (monitor.geometry(), monitor.scale()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(scale: f64, transform: u32) -> Head {
        Head {
            name: "DP-1".into(),
            enabled: true,
            mode: Some(asena_wayland::output::Mode {
                width: 2560,
                height: 1440,
                refresh: 60_000,
                preferred: true,
            }),
            x: 1920,
            y: 0,
            transform,
            scale,
            ..Head::default()
        }
    }

    #[test]
    fn heads_give_logical_geometry() {
        let Some((area, scale)) = rect(&head(2.0, 0)) else {
            panic!("rect");
        };
        assert_eq!(
            (area.x(), area.y(), area.width(), area.height()),
            (1920, 0, 1280, 720)
        );
        assert_eq!(scale, 2.0);
        let Some((area, _)) = rect(&head(1.0, 1)) else {
            panic!("rect");
        };
        assert_eq!((area.width(), area.height()), (1440, 2560));
    }

    #[test]
    fn disabled_or_modeless_heads_fall_back() {
        let mut off = head(1.0, 0);
        off.enabled = false;
        assert!(rect(&off).is_none());
        let mut modeless = head(1.0, 0);
        modeless.mode = None;
        assert!(rect(&modeless).is_none());
    }
}
