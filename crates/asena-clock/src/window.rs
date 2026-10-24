//! Clocks window: world clocks, stopwatch and timer.
use crate::clock::{
    city_of, offset_label, parse_saved, parse_zone_table, stopwatch_label, timer_label,
    zone_matches, Zone,
};
use adw::prelude::*;
use asena_app::{prelude::*, storage};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

fn saved_path() -> PathBuf {
    storage::data_dir("clock").join("world-clocks.txt")
}

#[derive(Default)]
struct Stopwatch {
    started: Cell<Option<Instant>>,
    elapsed: Cell<Duration>,
    laps: RefCell<Vec<Duration>>,
    tick: RefCell<Option<glib::SourceId>>,
}

impl Stopwatch {
    fn total(&self) -> Duration {
        self.elapsed.get()
            + self
                .started
                .get()
                .map(|start| start.elapsed())
                .unwrap_or_default()
    }
}

#[derive(Default)]
struct Timer {
    deadline: Cell<Option<Instant>>,
    remaining: Cell<Duration>,
    tick: RefCell<Option<glib::SourceId>>,
}

struct State {
    app: adw::Application,
    window: AsenaWindow,
    world: gtk::ListBox,
    saved: RefCell<Vec<String>>,
    zones: RefCell<Vec<Zone>>,
    minute: RefCell<Option<glib::SourceId>>,
    stopwatch: Stopwatch,
    stopwatch_label: gtk::Label,
    stopwatch_start: gtk::Button,
    stopwatch_lap: gtk::Button,
    laps: gtk::ListBox,
    timer: Timer,
    timer_stack: gtk::Stack,
    timer_label: gtk::Label,
    timer_start: gtk::Button,
    spins: [gtk::SpinButton; 3],
}

#[derive(Clone)]
pub struct ClockWindow {
    state: Rc<State>,
}

fn big_label() -> gtk::Label {
    let label = gtk::Label::new(None);
    label.add_css_class("title-1");
    label.add_css_class("numeric");
    label.add_css_class("asena-clock-display");
    label
}

impl ClockWindow {
    pub fn new(app: &adw::Application) -> Self {
        let stack = adw::ViewStack::new();

        // World ----------------------------------------------------------------
        let world = gtk::ListBox::new();
        world.add_css_class("boxed-list");
        world.set_selection_mode(gtk::SelectionMode::None);
        world.set_valign(gtk::Align::Start);
        let add_zone = gtk::Button::with_label(&gettext("Add city"));
        add_zone.add_css_class("pill");
        add_zone.set_halign(gtk::Align::Center);
        add_zone.set_action_name(Some("win.add-zone"));
        let world_box = gtk::Box::new(gtk::Orientation::Vertical, 18);
        world_box.set_margin_top(24);
        world_box.set_margin_bottom(24);
        world_box.set_margin_start(12);
        world_box.set_margin_end(12);
        world_box.append(&world);
        world_box.append(&add_zone);
        stack.add_titled_with_icon(
            &gtk::ScrolledWindow::builder()
                .child(
                    &adw::Clamp::builder()
                        .maximum_size(560)
                        .child(&world_box)
                        .build(),
                )
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("world"),
            &gettext("World"),
            "web-browser-symbolic",
        );

        // Stopwatch ------------------------------------------------------------
        let stopwatch_label = big_label();
        stopwatch_label.set_text(&stopwatch_label_text(0));
        let stopwatch_start = gtk::Button::with_label(&gettext("Start"));
        stopwatch_start.add_css_class("pill");
        stopwatch_start.add_css_class("suggested-action");
        let stopwatch_lap = gtk::Button::with_label(&gettext("Lap"));
        stopwatch_lap.add_css_class("pill");
        let stopwatch_reset = gtk::Button::with_label(&gettext("Reset"));
        stopwatch_reset.add_css_class("pill");
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_halign(gtk::Align::Center);
        buttons.append(&stopwatch_reset);
        buttons.append(&stopwatch_start);
        buttons.append(&stopwatch_lap);
        let laps = gtk::ListBox::new();
        laps.add_css_class("boxed-list");
        laps.set_selection_mode(gtk::SelectionMode::None);
        laps.set_visible(false);
        let stopwatch_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
        stopwatch_box.set_margin_top(36);
        stopwatch_box.set_margin_bottom(24);
        stopwatch_box.set_margin_start(12);
        stopwatch_box.set_margin_end(12);
        stopwatch_box.append(&stopwatch_label);
        stopwatch_box.append(&buttons);
        stopwatch_box.append(&laps);
        stack.add_titled_with_icon(
            &gtk::ScrolledWindow::builder()
                .child(
                    &adw::Clamp::builder()
                        .maximum_size(480)
                        .child(&stopwatch_box)
                        .build(),
                )
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("stopwatch"),
            &gettext("Stopwatch"),
            "media-playback-start-symbolic",
        );

        // Timer ------------------------------------------------------------------
        let spins = [
            gtk::SpinButton::with_range(0.0, 23.0, 1.0),
            gtk::SpinButton::with_range(0.0, 59.0, 1.0),
            gtk::SpinButton::with_range(0.0, 59.0, 1.0),
        ];
        spins[1].set_value(5.0);
        let setup = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        setup.set_halign(gtk::Align::Center);
        for (index, (spin, label)) in spins
            .iter()
            .zip([gettext("hours"), gettext("min"), gettext("sec")])
            .enumerate()
        {
            if index > 0 {
                setup.append(&gtk::Label::new(Some(":")));
            }
            let column = gtk::Box::new(gtk::Orientation::Vertical, 4);
            spin.set_orientation(gtk::Orientation::Vertical);
            spin.set_numeric(true);
            spin.add_css_class("asena-clock-spin");
            column.append(spin);
            let caption = gtk::Label::new(Some(&label));
            caption.add_css_class("dim-label");
            caption.add_css_class("caption");
            column.append(&caption);
            setup.append(&column);
        }
        let timer_label = big_label();
        let timer_stack = gtk::Stack::new();
        timer_stack.add_named(&setup, Some("setup"));
        timer_stack.add_named(&timer_label, Some("running"));
        let timer_start = gtk::Button::with_label(&gettext("Start"));
        timer_start.add_css_class("pill");
        timer_start.add_css_class("suggested-action");
        let timer_reset = gtk::Button::with_label(&gettext("Reset"));
        timer_reset.add_css_class("pill");
        let timer_buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        timer_buttons.set_halign(gtk::Align::Center);
        timer_buttons.append(&timer_reset);
        timer_buttons.append(&timer_start);
        let timer_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
        timer_box.set_margin_top(36);
        timer_box.set_margin_start(12);
        timer_box.set_margin_end(12);
        timer_box.append(&timer_stack);
        timer_box.append(&timer_buttons);
        stack.add_titled_with_icon(
            &timer_box,
            Some("timer"),
            &gettext("Timer"),
            "preferences-system-time-symbolic",
        );

        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&switcher));
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("About Clocks")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let bottom = adw::ViewSwitcherBar::new();
        bottom.set_stack(Some(&stack));
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        toolbar.add_bottom_bar(&bottom);

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Clocks"))
            .default_size(640, 560)
            .min_size(360, 400)
            .content(&toolbar)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            520.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(
            &header,
            "title-widget",
            Some(&None::<gtk::Widget>.to_value()),
        );
        breakpoint.add_setter(&bottom, "reveal", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            app: app.clone(),
            window,
            world,
            saved: RefCell::default(),
            zones: RefCell::default(),
            minute: RefCell::default(),
            stopwatch: Stopwatch::default(),
            stopwatch_label,
            stopwatch_start,
            stopwatch_lap,
            laps,
            timer: Timer::default(),
            timer_stack,
            timer_label,
            timer_start,
            spins,
        });
        let this = Self { state };
        let cloned = this.clone();
        stopwatch_reset.connect_clicked(move |_| cloned.stopwatch_reset());
        let cloned = this.clone();
        this.state
            .stopwatch_start
            .connect_clicked(move |_| cloned.stopwatch_toggle());
        let cloned = this.clone();
        this.state
            .stopwatch_lap
            .connect_clicked(move |_| cloned.stopwatch_lap());
        let cloned = this.clone();
        timer_reset.connect_clicked(move |_| cloned.timer_reset());
        let cloned = this.clone();
        this.state
            .timer_start
            .connect_clicked(move |_| cloned.timer_toggle());
        let action = gio::SimpleAction::new("add-zone", None);
        let cloned = this.clone();
        action.connect_activate(move |_, _| cloned.choose_zone());
        this.state.window.add_action(&action);
        this.stopwatch_sync();
        this.load_world();
        let cloned = this.clone();
        this.state
            .window
            .connect_map(move |_| cloned.schedule_minute());
        let cloned = this.clone();
        this.state.window.connect_unmap(move |_| {
            if let Some(source) = cloned.state.minute.borrow_mut().take() {
                source.remove();
            }
        });
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    // World clocks ----------------------------------------------------------------
    fn load_world(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let (saved, table) = gio::spawn_blocking(|| {
                let saved = std::fs::read_to_string(saved_path()).unwrap_or_default();
                let table =
                    std::fs::read_to_string("/usr/share/zoneinfo/zone1970.tab").unwrap_or_default();
                (parse_saved(&saved), parse_zone_table(&table))
            })
            .await
            .unwrap_or_default();
            this.state.saved.replace(saved);
            this.state.zones.replace(table);
            this.render_world();
        });
    }

    fn save_world(&self) {
        let text = self.state.saved.borrow().join("\n") + "\n";
        glib::spawn_future_local(async move {
            let result =
                gio::spawn_blocking(move || storage::write_atomic(&saved_path(), text.as_bytes()))
                    .await;
            if let Ok(Err(error)) = result {
                tracing::warn!(%error, "Could not save world clocks");
            }
        });
    }

    fn schedule_minute(&self) {
        self.render_world();
        if let Some(source) = self.state.minute.borrow_mut().take() {
            source.remove();
        }
        let seconds = glib::DateTime::now_local()
            .map(|now| 60 - now.second() as u64)
            .unwrap_or(60);
        let this = self.clone();
        let source = glib::timeout_add_local_once(Duration::from_secs(seconds.max(1)), move || {
            this.state.minute.borrow_mut().take();
            this.schedule_minute();
        });
        self.state.minute.replace(Some(source));
    }

    fn zone_row(&self, id: Option<&str>) -> Option<adw::ActionRow> {
        let zone = match id {
            // Unknown identifiers have no zone; skip those.
            Some(id) => glib::TimeZone::from_identifier(Some(id))?,
            None => glib::TimeZone::local(),
        };
        let now = glib::DateTime::now(&zone).ok()?;
        let local = glib::DateTime::now_local().ok()?;
        let row = adw::ActionRow::new();
        row.set_title(&match id {
            Some(id) => glib::markup_escape_text(&city_of(id)).to_string(),
            None => gettext("Local time"),
        });
        let difference = now.utc_offset().as_seconds() - local.utc_offset().as_seconds();
        let day = if now.ymd() == local.ymd() {
            String::new()
        } else if (now.year(), now.day_of_year()) > (local.year(), local.day_of_year()) {
            gettext("Tomorrow")
        } else {
            gettext("Yesterday")
        };
        let offset = offset_label(
            difference,
            &gettext("Same time"),
            &gettext("h"),
            &gettext("min"),
        );
        row.set_subtitle(&if day.is_empty() || id.is_none() {
            if id.is_none() {
                now.timezone_abbreviation().to_string()
            } else {
                offset
            }
        } else {
            format!("{day} · {offset}")
        });
        let time = gtk::Label::new(now.format("%H:%M").ok().as_deref());
        time.add_css_class("title-2");
        time.add_css_class("numeric");
        row.add_suffix(&time);
        if let Some(id) = id {
            let remove = gtk::Button::from_icon_name("user-trash-symbolic");
            remove.add_css_class("flat");
            remove.set_valign(gtk::Align::Center);
            remove.set_tooltip_text(Some(&gettext("Remove")));
            let this = self.clone();
            let id = id.to_owned();
            remove.connect_clicked(move |_| {
                this.state.saved.borrow_mut().retain(|item| item != &id);
                this.save_world();
                this.render_world();
            });
            row.add_suffix(&remove);
        }
        Some(row)
    }

    fn render_world(&self) {
        let list = &self.state.world;
        list.remove_all();
        if let Some(row) = self.zone_row(None) {
            list.append(&row);
        }
        for id in self.state.saved.borrow().iter() {
            if let Some(row) = self.zone_row(Some(id)) {
                list.append(&row);
            }
        }
    }

    fn choose_zone(&self) {
        let dialog = adw::Dialog::new();
        dialog.set_title(&gettext("Add city"));
        dialog.set_content_width(420);
        dialog.set_content_height(520);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search cities")));
        search.set_margin_start(12);
        search.set_margin_end(12);
        search.set_margin_bottom(8);
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        let render = {
            let list = list.clone();
            let zones = self.state.zones.clone();
            move |query: &str| {
                list.remove_all();
                for zone in zones
                    .borrow()
                    .iter()
                    .filter(|zone| zone_matches(zone, query))
                    .take(200)
                {
                    let row = adw::ActionRow::new();
                    row.set_title(&glib::markup_escape_text(&zone.city));
                    row.set_subtitle(&glib::markup_escape_text(&zone.id));
                    row.set_activatable(true);
                    row.set_widget_name(&zone.id);
                    list.append(&row);
                }
            }
        };
        render("");
        let render = Rc::new(render);
        let update = render.clone();
        search.connect_search_changed(move |entry| update(&entry.text()));
        let this = self.clone();
        let weak = dialog.downgrade();
        list.connect_row_activated(move |_, row| {
            let id = row.widget_name().to_string();
            if !this.state.saved.borrow().contains(&id) {
                this.state.saved.borrow_mut().push(id);
                this.save_world();
                this.render_world();
            }
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        });
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.add_top_bar(&search);
        toolbar.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
        ));
        dialog.set_child(Some(&toolbar));
        dialog.present(Some(self.state.window.window()));
        search.grab_focus();
    }

    // Stopwatch ---------------------------------------------------------------------
    fn stopwatch_sync(&self) {
        let watch = &self.state.stopwatch;
        let running = watch.started.get().is_some();
        self.state
            .stopwatch_label
            .set_text(&stopwatch_label_text(watch.total().as_millis() as u64));
        self.state.stopwatch_start.set_label(&if running {
            gettext("Pause")
        } else if watch.total().is_zero() {
            gettext("Start")
        } else {
            gettext("Resume")
        });
        self.state.stopwatch_lap.set_sensitive(running);
    }

    fn stopwatch_toggle(&self) {
        let watch = &self.state.stopwatch;
        match watch.started.take() {
            Some(start) => {
                watch.elapsed.set(watch.elapsed.get() + start.elapsed());
                if let Some(source) = watch.tick.borrow_mut().take() {
                    source.remove();
                }
            }
            None => {
                watch.started.set(Some(Instant::now()));
                let this = self.clone();
                let source = glib::timeout_add_local(Duration::from_millis(40), move || {
                    this.stopwatch_sync();
                    glib::ControlFlow::Continue
                });
                watch.tick.replace(Some(source));
            }
        }
        self.stopwatch_sync();
    }

    fn stopwatch_lap(&self) {
        let total = self.state.stopwatch.total();
        let mut laps = self.state.stopwatch.laps.borrow_mut();
        let previous: Duration = laps.iter().sum();
        laps.push(total.saturating_sub(previous));
        let row = adw::ActionRow::new();
        row.set_title(&gettext("Lap {number}").replace("{number}", &laps.len().to_string()));
        let lap = gtk::Label::new(Some(&stopwatch_label_text(
            total.saturating_sub(previous).as_millis() as u64,
        )));
        lap.add_css_class("numeric");
        row.add_suffix(&lap);
        let overall = gtk::Label::new(Some(&stopwatch_label_text(total.as_millis() as u64)));
        overall.add_css_class("numeric");
        overall.add_css_class("dim-label");
        row.add_suffix(&overall);
        self.state.laps.prepend(&row);
        self.state.laps.set_visible(true);
    }

    fn stopwatch_reset(&self) {
        let watch = &self.state.stopwatch;
        if let Some(source) = watch.tick.borrow_mut().take() {
            source.remove();
        }
        watch.started.set(None);
        watch.elapsed.set(Duration::ZERO);
        watch.laps.borrow_mut().clear();
        self.state.laps.remove_all();
        self.state.laps.set_visible(false);
        self.stopwatch_sync();
    }

    // Timer ---------------------------------------------------------------------------
    fn timer_toggle(&self) {
        let timer = &self.state.timer;
        if let Some(deadline) = timer.deadline.take() {
            timer
                .remaining
                .set(deadline.saturating_duration_since(Instant::now()));
            if let Some(source) = timer.tick.borrow_mut().take() {
                source.remove();
            }
            self.state.timer_start.set_label(&gettext("Resume"));
            return;
        }
        if timer.remaining.get().is_zero() {
            let [hours, minutes, seconds] = &self.state.spins;
            let total = hours.value_as_int() as u64 * 3600
                + minutes.value_as_int() as u64 * 60
                + seconds.value_as_int() as u64;
            if total == 0 {
                return;
            }
            timer.remaining.set(Duration::from_secs(total));
        }
        timer
            .deadline
            .set(Some(Instant::now() + timer.remaining.get()));
        self.state.timer_stack.set_visible_child_name("running");
        self.state.timer_start.set_label(&gettext("Pause"));
        self.timer_tick();
        let this = self.clone();
        let source = glib::timeout_add_local(Duration::from_millis(200), move || {
            if this.timer_tick() {
                glib::ControlFlow::Continue
            } else {
                this.state.timer.tick.borrow_mut().take();
                glib::ControlFlow::Break
            }
        });
        timer.tick.replace(Some(source));
    }

    /// Updates the display; returns false once the timer has finished.
    fn timer_tick(&self) -> bool {
        let Some(deadline) = self.state.timer.deadline.get() else {
            return false;
        };
        let left = deadline.saturating_duration_since(Instant::now());
        self.state
            .timer_label
            .set_text(&timer_label(left.as_millis() as u64));
        if left.is_zero() {
            self.finish_timer();
            return false;
        }
        true
    }

    fn finish_timer(&self) {
        let timer = &self.state.timer;
        timer.deadline.set(None);
        timer.remaining.set(Duration::ZERO);
        self.state.timer_stack.set_visible_child_name("setup");
        self.state.timer_start.set_label(&gettext("Start"));
        let notification = gio::Notification::new(&gettext("Timer finished"));
        notification.set_priority(gio::NotificationPriority::High);
        self.state
            .app
            .send_notification(Some("timer"), &notification);
        self.state.window.show_toast(&gettext("Timer finished"));
    }

    fn timer_reset(&self) {
        let timer = &self.state.timer;
        if let Some(source) = timer.tick.borrow_mut().take() {
            source.remove();
        }
        timer.deadline.set(None);
        timer.remaining.set(Duration::ZERO);
        self.state.timer_stack.set_visible_child_name("setup");
        self.state.timer_start.set_label(&gettext("Start"));
    }
}

fn stopwatch_label_text(millis: u64) -> String {
    stopwatch_label(millis)
}
