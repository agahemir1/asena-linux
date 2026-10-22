//! Calendar window: month view, the selected day's events and an event editor.
use crate::ics::{self, Date, Event, Time};
use adw::prelude::*;
use asena_app::{prelude::*, storage};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

struct State {
    window: AsenaWindow,
    calendar: gtk::Calendar,
    heading: gtk::Label,
    list: gtk::ListBox,
    day_stack: gtk::Stack,
    events: RefCell<Vec<Event>>,
}

#[derive(Clone)]
pub struct CalendarWindow {
    state: Rc<State>,
}

fn store_path() -> PathBuf {
    storage::data_dir("calendar").join("events.ics")
}

fn utc_to_local(date: Date, time: Time) -> Option<(Date, Time)> {
    let utc = glib::DateTime::from_utc(
        date.year,
        date.month as i32,
        date.day as i32,
        time.hour as i32,
        time.minute as i32,
        0.0,
    )
    .ok()?;
    let local = utc.to_local().ok()?;
    Some((
        Date {
            year: local.year(),
            month: local.month() as u32,
            day: local.day_of_month() as u32,
        },
        Time {
            hour: local.hour() as u32,
            minute: local.minute() as u32,
        },
    ))
}

fn selected(calendar: &gtk::Calendar) -> Date {
    let date = calendar.date();
    Date {
        year: date.year(),
        month: date.month() as u32,
        day: date.day_of_month() as u32,
    }
}

impl CalendarWindow {
    pub fn new(app: &adw::Application) -> Self {
        let calendar = gtk::Calendar::new();
        calendar.set_show_heading(true);
        calendar.set_show_day_names(true);
        calendar.set_valign(gtk::Align::Start);
        calendar.add_css_class("card");

        let heading = gtk::Label::new(None);
        heading.set_xalign(0.0);
        heading.add_css_class("title-3");
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::None);
        list.set_valign(gtk::Align::Start);
        let empty = gtk::Label::new(Some(&gettext("No events on this day")));
        empty.add_css_class("dim-label");
        empty.set_margin_top(24);
        let day_stack = gtk::Stack::new();
        day_stack.add_named(&list, Some("events"));
        day_stack.add_named(&empty, Some("empty"));
        let add = gtk::Button::with_label(&gettext("Add event"));
        add.add_css_class("pill");
        add.set_halign(gtk::Align::Start);
        add.set_action_name(Some("win.add-event"));
        let day = gtk::Box::new(gtk::Orientation::Vertical, 12);
        day.set_hexpand(true);
        day.append(&heading);
        day.append(&day_stack);
        day.append(&add);

        let layout = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        layout.set_margin_top(18);
        layout.set_margin_bottom(18);
        layout.set_margin_start(18);
        layout.set_margin_end(18);
        layout.append(&calendar);
        layout.append(&day);
        let scroll = gtk::ScrolledWindow::builder()
            .child(
                &adw::Clamp::builder()
                    .maximum_size(960)
                    .child(&layout)
                    .build(),
            )
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let header = adw::HeaderBar::new();
        let today = gtk::Button::with_label(&gettext("Today"));
        today.set_action_name(Some("win.today"));
        header.pack_start(&today);
        let menu = gio::Menu::new();
        let files = gio::Menu::new();
        files.append(Some(&gettext("Import events…")), Some("win.import"));
        files.append(Some(&gettext("Export events…")), Some("win.export"));
        menu.append_section(None, &files);
        menu.append(Some(&gettext("About Calendar")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&scroll));

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Calendar"))
            .default_size(860, 560)
            .min_size(360, 420)
            .content(&toolbar)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            640.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(
            &layout,
            "orientation",
            Some(&gtk::Orientation::Vertical.to_value()),
        );
        breakpoint.add_setter(&calendar, "halign", Some(&gtk::Align::Center.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            window,
            calendar,
            heading,
            list,
            day_stack,
            events: RefCell::default(),
        });
        let this = Self { state };
        this.setup();
        this.load();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn setup(&self) {
        let window = &self.state.window;
        let add = |name: &str, accels: &[&str], callback: Box<dyn Fn(&Self)>| {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| callback(&this));
            window.add_action(&action);
            if let Some(app) = window.application() {
                app.set_accels_for_action(&format!("win.{name}"), accels);
            }
        };
        add(
            "add-event",
            &["<Control>n"],
            Box::new(|this| this.edit(None)),
        );
        add(
            "today",
            &["<Control>t"],
            Box::new(|this| {
                if let Ok(now) = glib::DateTime::now_local() {
                    this.state.calendar.select_day(&now);
                }
            }),
        );
        add("import", &[], Box::new(|this| this.import()));
        add("export", &[], Box::new(|this| this.export()));
        let this = self.clone();
        self.state
            .calendar
            .connect_day_selected(move |_| this.render_day());
        let this = self.clone();
        self.state
            .calendar
            .connect_month_notify(move |_| this.mark_days());
        let this = self.clone();
        self.state
            .calendar
            .connect_year_notify(move |_| this.mark_days());
    }

    fn load(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let text = gio::spawn_blocking(|| std::fs::read_to_string(store_path())).await;
            let events = match text {
                Ok(Ok(text)) => ics::parse(&text, &utc_to_local),
                Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                Ok(Err(error)) => {
                    show_error_dialog(
                        this.state.window.window(),
                        &gettext("Could not read the calendar"),
                        &error.to_string(),
                    );
                    Vec::new()
                }
                Err(_) => Vec::new(),
            };
            this.state.events.replace(events);
            this.mark_days();
            this.render_day();
        });
    }

    fn save(&self) {
        let text = ics::serialize(&self.state.events.borrow());
        let window = self.state.window.clone();
        glib::spawn_future_local(async move {
            let result =
                gio::spawn_blocking(move || storage::write_atomic(&store_path(), text.as_bytes()))
                    .await;
            if let Ok(Err(error)) = result {
                show_error_dialog(
                    window.window(),
                    &gettext("Could not save the calendar"),
                    &error.to_string(),
                );
            }
        });
        self.mark_days();
        self.render_day();
    }

    fn mark_days(&self) {
        let calendar = &self.state.calendar;
        calendar.clear_marks();
        let shown = selected(calendar);
        for event in self.state.events.borrow().iter() {
            if event.date.year == shown.year && event.date.month == shown.month {
                calendar.mark_day(event.date.day);
            }
        }
    }

    fn render_day(&self) {
        let date = selected(&self.state.calendar);
        let heading = self
            .state
            .calendar
            .date()
            .format("%e %B %Y, %A")
            .map(|text| text.trim().to_owned())
            .unwrap_or_default();
        self.state.heading.set_text(&heading);
        let list = &self.state.list;
        list.remove_all();
        let events = self.state.events.borrow();
        let day = ics::on_day(&events, date);
        self.state
            .day_stack
            .set_visible_child_name(if day.is_empty() { "empty" } else { "events" });
        for event in day {
            let row = adw::ActionRow::new();
            row.set_title(&glib::markup_escape_text(&event.title));
            let time = match (event.start, event.end) {
                (Some(start), Some(end)) if end > start => {
                    format!("{} – {}", start.label(), end.label())
                }
                (Some(start), _) => start.label(),
                (None, _) => gettext("All day"),
            };
            let subtitle = if event.location.is_empty() {
                time
            } else {
                format!("{time} · {}", event.location)
            };
            row.set_subtitle(&glib::markup_escape_text(&subtitle));
            row.set_activatable(true);
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            let this = self.clone();
            let uid = event.uid.clone();
            row.connect_activated(move |_| this.edit(Some(uid.clone())));
            list.append(&row);
        }
    }

    fn edit(&self, uid: Option<String>) {
        let existing = uid.as_ref().and_then(|uid| {
            self.state
                .events
                .borrow()
                .iter()
                .find(|event| &event.uid == uid)
                .cloned()
        });
        let date = existing
            .as_ref()
            .map(|event| event.date)
            .unwrap_or_else(|| selected(&self.state.calendar));
        let dialog = adw::AlertDialog::new(
            Some(&if existing.is_some() {
                gettext("Edit event")
            } else {
                gettext("New event")
            }),
            None,
        );
        let group = adw::PreferencesGroup::new();
        let title = adw::EntryRow::builder().title(gettext("Title")).build();
        let all_day = adw::SwitchRow::builder().title(gettext("All day")).build();
        let start = adw::EntryRow::builder()
            .title(gettext("Starts (HH:MM)"))
            .build();
        let end = adw::EntryRow::builder()
            .title(gettext("Ends (HH:MM)"))
            .build();
        let location = adw::EntryRow::builder().title(gettext("Location")).build();
        for row in [
            title.upcast_ref::<gtk::Widget>(),
            all_day.upcast_ref(),
            start.upcast_ref(),
            end.upcast_ref(),
            location.upcast_ref(),
        ] {
            group.add(row);
        }
        match &existing {
            Some(event) => {
                title.set_text(&event.title);
                all_day.set_active(event.start.is_none());
                start.set_text(&event.start.map(Time::label).unwrap_or_default());
                end.set_text(&event.end.map(Time::label).unwrap_or_default());
                location.set_text(&event.location);
            }
            None => {
                start.set_text("09:00");
                end.set_text("10:00");
            }
        }
        all_day
            .bind_property("active", &start, "sensitive")
            .invert_boolean()
            .sync_create()
            .build();
        all_day
            .bind_property("active", &end, "sensitive")
            .invert_boolean()
            .sync_create()
            .build();
        dialog.set_extra_child(Some(&group));
        dialog.add_response("cancel", &gettext("Cancel"));
        if existing.is_some() {
            dialog.add_response("delete", &gettext("Delete"));
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        }
        dialog.add_response("save", &gettext("Save"));
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let validate = {
            let dialog = dialog.downgrade();
            let (title, all_day, start, end) =
                (title.clone(), all_day.clone(), start.clone(), end.clone());
            move || {
                let times_ok = all_day.is_active()
                    || (Time::parse(&start.text()).is_some()
                        && (end.text().trim().is_empty() || Time::parse(&end.text()).is_some()));
                let valid = !title.text().trim().is_empty() && times_ok;
                if let Some(dialog) = dialog.upgrade() {
                    dialog.set_response_enabled("save", valid);
                }
            }
        };
        validate();
        let validate = Rc::new(validate);
        for entry in [&title, &start, &end] {
            let validate = validate.clone();
            entry.connect_changed(move |_| validate());
        }
        let refresh = validate.clone();
        all_day.connect_active_notify(move |_| refresh());
        let this = self.clone();
        dialog.connect_response(None, move |_, response| match response {
            "save" => {
                let start_time = (!all_day.is_active())
                    .then(|| Time::parse(&start.text()))
                    .flatten();
                let end_time = start_time.and_then(|_| Time::parse(&end.text()));
                let event = Event {
                    uid: uid
                        .clone()
                        .unwrap_or_else(|| format!("{}@asenalinux", glib::uuid_string_random())),
                    title: title.text().trim().to_owned(),
                    date,
                    start: start_time,
                    end: end_time,
                    location: location.text().trim().to_owned(),
                    notes: existing
                        .as_ref()
                        .map(|event| event.notes.clone())
                        .unwrap_or_default(),
                };
                {
                    let mut events = this.state.events.borrow_mut();
                    events.retain(|item| item.uid != event.uid);
                    events.push(event);
                }
                this.save();
            }
            "delete" => {
                if let Some(uid) = &uid {
                    this.state
                        .events
                        .borrow_mut()
                        .retain(|item| &item.uid != uid);
                    this.save();
                }
            }
            _ => {}
        });
        dialog.present(Some(self.state.window.window()));
    }

    fn import(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(&gettext("Calendar files")));
        filter.add_mime_type("text/calendar");
        filter.add_suffix("ics");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::new();
        dialog.set_filters(Some(&filters));
        let this = self.clone();
        dialog.open(
            Some(self.state.window.window()),
            gio::Cancellable::NONE,
            move |result| {
                let Ok(file) = result else {
                    return;
                };
                let this = this.clone();
                file.load_contents_async(gio::Cancellable::NONE, move |result| match result {
                    Ok((bytes, _)) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let imported = ics::parse(&text, &utc_to_local);
                        let count = imported.len();
                        {
                            let mut events = this.state.events.borrow_mut();
                            for event in imported {
                                events.retain(|item| item.uid != event.uid);
                                events.push(event);
                            }
                        }
                        this.save();
                        this.state.window.show_toast(
                            &asena_core::i18n::ngettext(
                                "{count} event imported",
                                "{count} events imported",
                                count as u32,
                            )
                            .replace("{count}", &count.to_string()),
                        );
                    }
                    Err(error) => show_error_dialog(
                        this.state.window.window(),
                        &gettext("Could not import the file"),
                        error.message(),
                    ),
                });
            },
        );
    }

    fn export(&self) {
        let dialog = gtk::FileDialog::new();
        dialog.set_initial_name(Some(&format!("{}.ics", gettext("calendar"))));
        let text = ics::serialize(&self.state.events.borrow());
        let window = self.state.window.clone();
        dialog.save(
            Some(self.state.window.window()),
            gio::Cancellable::NONE,
            move |result| {
                let Ok(file) = result else {
                    return;
                };
                let window = window.clone();
                file.replace_contents_async(
                    text.into_bytes(),
                    None,
                    false,
                    gio::FileCreateFlags::REPLACE_DESTINATION,
                    gio::Cancellable::NONE,
                    move |result| match result {
                        Ok(_) => window.show_toast(&gettext("Events exported")),
                        Err((_, error)) => show_error_dialog(
                            window.window(),
                            &gettext("Could not export the events"),
                            error.message(),
                        ),
                    },
                );
            },
        );
    }
}
