use asena_core::layout::PanelLayout;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type Timer = Rc<RefCell<Option<glib::SourceId>>>;

struct Labels {
    time: gtk::Label,
    date: gtk::Label,
}

/// Time format for the panel; vertical panels stack hours over minutes.
fn time_format(horizontal: bool, twelve: bool, seconds: bool) -> &'static str {
    match (horizontal, twelve, seconds) {
        (false, true, _) => "%I\n%M",
        (false, false, _) => "%H\n%M",
        (true, true, true) => "%I:%M:%S %p",
        (true, true, false) => "%I:%M %p",
        (true, false, true) => "%H:%M:%S",
        (true, false, false) => "%H:%M",
    }
}

fn tick(button: &gtk::Button, labels: &Rc<Labels>, settings: &gio::Settings, timer: &Timer) {
    if let Some(old) = timer.borrow_mut().take() {
        old.remove();
    }
    let Ok(now) = glib::DateTime::now_local() else {
        return;
    };
    let seconds = settings.boolean("clock-show-seconds");
    let layout = PanelLayout::from_settings(settings);
    let twelve = settings.string("clock-format") == "12h";
    if let Ok(text) = now.format(time_format(layout.is_horizontal(), twelve, seconds)) {
        labels.time.set_text(&text);
    }
    // A second line for the date only where the panel has room for it.
    let show_date = layout.is_horizontal() && layout.thickness >= 44;
    labels.date.set_visible(show_date);
    if let Ok(date) = now.format("%e %b") {
        labels.date.set_text(date.trim());
    }
    if let Ok(date) = now.format("%A, %e %B %Y") {
        button.set_tooltip_text(Some(date.trim()));
    }
    let delay =
        asena_core::taskbar::clock_delay_ms(now.second() as u32, now.microsecond() as u32, seconds);
    let weak = button.downgrade();
    let labels = labels.clone();
    let settings = settings.clone();
    let pending = timer.clone();
    let id = glib::timeout_add_local_once(Duration::from_millis(delay), move || {
        pending.borrow_mut().take();
        if let Some(button) = weak.upgrade() {
            tick(&button, &labels, &settings, &pending);
        }
    });
    timer.replace(Some(id));
}

pub fn create(settings: &gio::Settings) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("flat");
    button.add_css_class("asena-clock");
    let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    column.set_valign(gtk::Align::Center);
    let labels = Rc::new(Labels {
        time: gtk::Label::new(None),
        date: gtk::Label::new(None),
    });
    labels.time.add_css_class("asena-clock-time");
    labels.time.add_css_class("numeric");
    labels.time.set_justify(gtk::Justification::Center);
    labels.date.add_css_class("asena-clock-date");
    column.append(&labels.time);
    column.append(&labels.date);
    button.set_child(Some(&column));
    let timer = Timer::default();
    tick(&button, &labels, settings, &timer);
    let weak = button.downgrade();
    let pending = timer.clone();
    let active = labels.clone();
    let handler = settings.connect_changed(None, move |settings, key| {
        if !matches!(
            key,
            "clock-show-seconds" | "clock-format" | "panel-edge" | "panel-thickness"
        ) {
            return;
        }
        if let Some(button) = weak.upgrade() {
            tick(&button, &active, settings, &pending);
        }
    });
    let settings = settings.clone();
    let handler = RefCell::new(Some(handler));
    button.connect_destroy(move |_| {
        if let Some(id) = timer.borrow_mut().take() {
            id.remove();
        }
        if let Some(id) = handler.borrow_mut().take() {
            settings.disconnect(id);
        }
    });
    button
}

#[cfg(test)]
mod tests {
    use super::time_format;

    #[test]
    fn vertical_panels_stack_hours_and_minutes() {
        assert_eq!(time_format(false, false, true), "%H\n%M");
        assert_eq!(time_format(true, false, false), "%H:%M");
        assert_eq!(time_format(true, true, true), "%I:%M:%S %p");
    }
}
