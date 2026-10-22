use adw::prelude::*;
use asena_core::i18n::gettext;
use std::{cell::Cell, rc::Rc};
pub fn error(label: &gtk::Label, text: &str) {
    label.set_text(text);
    label.set_visible(!text.is_empty());
}
pub fn boolean(group: &adw::PreferencesGroup, settings: &gio::Settings, key: &str, title: &str) {
    let row = adw::SwitchRow::builder().title(title).build();
    settings.bind(key, &row, "active").build();
    group.add(&row);
}
pub fn choice(
    group: &adw::PreferencesGroup,
    settings: &gio::Settings,
    key: &str,
    title: &str,
    items: &[(&str, String)],
) -> adw::ComboRow {
    let values = Rc::new(
        items
            .iter()
            .map(|(key, _)| key.to_string())
            .collect::<Vec<_>>(),
    );
    let labels: Vec<_> = items.iter().map(|(_, label)| label.as_str()).collect();
    let row = adw::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&labels))
        .build();
    row.set_selected(
        values
            .iter()
            .position(|value| value == settings.string(key).as_str())
            .map(|index| index as u32)
            .unwrap_or(gtk::INVALID_LIST_POSITION),
    );
    let changing = Rc::new(Cell::new(false));
    let key_owned = key.to_owned();
    let prefs = settings.clone();
    let selected = values.clone();
    let guard = changing.clone();
    row.connect_selected_notify(move |row| {
        if guard.get() {
            return;
        }
        if let Some(value) = selected.get(row.selected() as usize) {
            if let Err(error) = prefs.set_string(&key_owned, value) {
                tracing::warn!(%error,"Could not save a preference");
            }
        }
    });
    let weak = row.downgrade();
    let key_owned = key.to_owned();
    let selected = values;
    settings.connect_changed(Some(key), move |prefs, _| {
        if let Some(row) = weak.upgrade() {
            changing.set(true);
            row.set_selected(
                selected
                    .iter()
                    .position(|value| value == prefs.string(&key_owned).as_str())
                    .map(|index| index as u32)
                    .unwrap_or(gtk::INVALID_LIST_POSITION),
            );
            changing.set(false);
        }
    });
    group.add(&row);
    row
}
pub fn number(
    group: &adw::PreferencesGroup,
    settings: &gio::Settings,
    key: &str,
    title: &str,
    range: (f64, f64, f64),
    error_label: &gtk::Label,
) {
    let value = || {
        settings
            .value(key)
            .get::<i32>()
            .map(f64::from)
            .or_else(|| settings.value(key).get::<u32>().map(f64::from))
            .unwrap_or_else(|| settings.double(key))
    };
    let row = adw::SpinRow::with_range(range.0, range.1, range.2);
    row.set_title(title);
    row.set_value(value());
    if range.2 < 1.0 {
        row.set_digits(2);
    }
    let key = key.to_owned();
    let prefs = settings.clone();
    let label = error_label.clone();
    row.connect_value_notify(move |row| {
        let value = row.value();
        let valid = match key.as_str() {
            "panel-thickness" => asena_core::layout::PanelLayout::valid_dimensions(
                value as i32,
                prefs.int("icon-size"),
            ),
            "icon-size" => asena_core::layout::PanelLayout::valid_dimensions(
                prefs.int("panel-thickness"),
                value as i32,
            ),
            _ => true,
        };
        if !valid {
            error(
                &label,
                &gettext("The panel and icon sizes are incompatible."),
            );
            return;
        }
        let current = prefs.value(&key);
        let result = if current.is_type(glib::VariantTy::INT32) {
            prefs.set_int(&key, value as i32)
        } else if current.is_type(glib::VariantTy::UINT32) {
            prefs.set_uint(&key, value as u32)
        } else {
            prefs.set_double(&key, value)
        };
        if let Err(failure) = result {
            error(&label, &failure.to_string());
        } else {
            error(&label, "");
        }
    });
    group.add(&row);
}
pub fn button(group: &adw::PreferencesGroup, title: &str, label: &str) -> gtk::Button {
    let row = adw::ActionRow::builder().title(title).build();
    let button = gtk::Button::with_label(label);
    button.set_valign(gtk::Align::Center);
    row.add_suffix(&button);
    row.set_activatable_widget(Some(&button));
    group.add(&row);
    button
}
/// A read-only row; the returned label can be updated later.
pub fn info(group: &adw::PreferencesGroup, title: &str, value: &str) -> gtk::Label {
    let row = adw::ActionRow::builder().title(title).build();
    let label = gtk::Label::new(Some(value));
    label.add_css_class("dim-label");
    label.set_selectable(true);
    label.set_wrap(true);
    label.set_xalign(1.0);
    label.set_justify(gtk::Justification::Right);
    label.set_max_width_chars(36);
    row.add_suffix(&label);
    group.add(&row);
    label
}
