//! Weather window: current conditions, the next hours and the week for saved places.
use crate::{
    forecast::{self, Forecast, Units},
    places::{self, Place, Saved},
};
use adw::prelude::*;
use asena_app::{prelude::*, storage};
use soup::prelude::*;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

/// A forecast younger than this is shown without asking the service again.
const FRESH_SECONDS: i64 = 15 * 60;
/// Forecast and search answers are a few kilobytes.
const LIMIT: usize = 1024 * 1024;

fn saved_path() -> PathBuf {
    storage::data_dir("weather").join("places.json")
}

fn cache_path(place: &Place, units: Units) -> PathBuf {
    glib::user_cache_dir().join("asena/weather").join(format!(
        "{:.4}_{:.4}_{}.json",
        place.latitude,
        place.longitude,
        units.id()
    ))
}

thread_local! {
    static SESSION: soup::Session = {
        let session = soup::Session::new();
        session.set_timeout(20);
        session.set_user_agent(concat!(
            "Asena Weather/",
            env!("CARGO_PKG_VERSION"),
            " (+https://asenalinux.netlify.app)"
        ));
        session
    };
}

async fn fetch(url: &str) -> Result<String, String> {
    let message = soup::Message::new("GET", url).map_err(|error| error.to_string())?;
    let session = SESSION.with(Clone::clone);
    let bytes = session
        .send_and_read_future(&message, glib::Priority::DEFAULT)
        .await
        .map_err(|error| error.to_string())?;
    if message.status() != soup::Status::Ok {
        return Err(format!("HTTP {:?}", message.status()));
    }
    if bytes.len() > LIMIT {
        return Err("answer too large".into());
    }
    String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
}

fn now_unix() -> i64 {
    glib::DateTime::now_utc()
        .map(|now| now.to_unix())
        .unwrap_or_default()
}

fn time_of(unix: i64) -> String {
    glib::DateTime::from_unix_local(unix)
        .and_then(|time| time.format("%H:%M"))
        .map(|text| text.to_string())
        .unwrap_or_default()
}

/// `YYYY-MM-DD` → the localized weekday name.
fn weekday(date: &str) -> String {
    let mut parts = date.split('-').map(|part| part.parse::<i32>().unwrap_or(0));
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return date.to_owned();
    };
    glib::DateTime::new(&glib::TimeZone::utc(), year, month, day, 12, 0, 0.0)
        .and_then(|time| time.format("%A"))
        .map(|text| text.to_string())
        .unwrap_or_else(|_| date.to_owned())
}

fn dim(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("dim-label");
    label
}

fn detail_row(list: &gtk::ListBox, title: &str) -> gtk::Label {
    let row = adw::ActionRow::builder().title(title).build();
    let value = gtk::Label::new(None);
    value.add_css_class("numeric");
    row.add_suffix(&value);
    list.append(&row);
    value
}

struct State {
    window: AsenaWindow,
    saved: RefCell<Saved>,
    stack: gtk::Stack,
    title: adw::WindowTitle,
    places: gtk::ListBox,
    places_popover: gtk::Popover,
    error: adw::StatusPage,
    icon: gtk::Image,
    temperature: gtk::Label,
    summary: gtk::Label,
    feels_like: gtk::Label,
    hours: gtk::Box,
    days: gtk::ListBox,
    humidity: gtk::Label,
    wind: gtk::Label,
    pressure: gtk::Label,
    sunrise: gtk::Label,
    sunset: gtk::Label,
    updated: gtk::Label,
    /// Answers for an earlier place or unit choice are dropped.
    generation: Cell<u64>,
    shown: Cell<bool>,
}

#[derive(Clone)]
pub struct WeatherWindow {
    state: Rc<State>,
}

impl WeatherWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Current conditions ---------------------------------------------------------
        let icon = gtk::Image::new();
        icon.set_pixel_size(96);
        icon.add_css_class("dim-label");
        let temperature = gtk::Label::new(None);
        temperature.add_css_class("title-1");
        temperature.add_css_class("numeric");
        temperature.set_xalign(0.0);
        let summary = gtk::Label::new(None);
        summary.add_css_class("title-4");
        summary.set_xalign(0.0);
        summary.set_wrap(true);
        let feels_like = dim("");
        feels_like.set_xalign(0.0);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
        text.set_valign(gtk::Align::Center);
        text.append(&temperature);
        text.append(&summary);
        text.append(&feels_like);
        let current = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        current.set_halign(gtk::Align::Center);
        current.append(&icon);
        current.append(&text);

        // The next hours --------------------------------------------------------------
        let hours = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let hours_scroll = gtk::ScrolledWindow::builder()
            .child(&hours)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .build();
        hours_scroll.add_css_class("card");

        // The week and details ----------------------------------------------------------
        let days = gtk::ListBox::new();
        days.add_css_class("boxed-list");
        days.set_selection_mode(gtk::SelectionMode::None);
        let details = gtk::ListBox::new();
        details.add_css_class("boxed-list");
        details.set_selection_mode(gtk::SelectionMode::None);
        let humidity = detail_row(&details, &gettext("Humidity"));
        let wind = detail_row(&details, &gettext("Wind"));
        let pressure = detail_row(&details, &gettext("Pressure"));
        let sunrise = detail_row(&details, &gettext("Sunrise"));
        let sunset = detail_row(&details, &gettext("Sunset"));

        let heading = |text: &str| {
            let label = gtk::Label::new(Some(text));
            label.add_css_class("heading");
            label.set_xalign(0.0);
            label.set_margin_top(6);
            label
        };
        let updated = dim("");
        updated.add_css_class("caption");
        let source = gtk::LinkButton::with_label(forecast::ATTRIBUTION_URL, "Open-Meteo.com");
        source.add_css_class("caption");
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        footer.set_halign(gtk::Align::Center);
        footer.append(&updated);
        footer.append(&dim(&gettext("Weather data:")));
        footer.append(&source);
        let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
        column.set_margin_top(18);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        column.append(&current);
        column.append(&heading(&gettext("Hourly")));
        column.append(&hours_scroll);
        column.append(&heading(&gettext("7 days")));
        column.append(&days);
        column.append(&heading(&gettext("Details")));
        column.append(&details);
        column.append(&footer);
        let forecast_view = gtk::ScrolledWindow::builder()
            .child(
                &adw::Clamp::builder()
                    .maximum_size(560)
                    .child(&column)
                    .build(),
            )
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let empty = adw::StatusPage::builder()
            .icon_name("find-location-symbolic")
            .title(gettext("No places"))
            .build();
        let add = gtk::Button::with_label(&gettext("Add place"));
        add.add_css_class("pill");
        add.add_css_class("suggested-action");
        add.set_halign(gtk::Align::Center);
        add.set_action_name(Some("win.add-place"));
        empty.set_child(Some(&add));
        let error = adw::StatusPage::builder()
            .icon_name("network-offline-symbolic")
            .title(gettext("Could not load the forecast"))
            .build();
        let retry = gtk::Button::with_label(&gettext("Try Again"));
        retry.add_css_class("pill");
        retry.set_halign(gtk::Align::Center);
        retry.set_action_name(Some("win.refresh"));
        error.set_child(Some(&retry));
        let loading = adw::Spinner::new();
        loading.set_size_request(48, 48);
        loading.set_halign(gtk::Align::Center);
        loading.set_valign(gtk::Align::Center);

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&error, Some("error"));
        stack.add_named(&forecast_view, Some("forecast"));
        stack.set_visible_child_name("loading");

        // Header ---------------------------------------------------------------------------
        let title = adw::WindowTitle::new(&gettext("Weather"), "");
        let places = gtk::ListBox::new();
        places.add_css_class("navigation-sidebar");
        let add_place = gtk::Button::with_label(&gettext("Add place"));
        add_place.add_css_class("flat");
        add_place.set_action_name(Some("win.add-place"));
        let popover_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        popover_box.append(&places);
        popover_box.append(&add_place);
        let places_popover = gtk::Popover::new();
        places_popover.set_child(Some(&popover_box));
        let places_button = gtk::MenuButton::new();
        places_button.set_icon_name("find-location-symbolic");
        places_button.set_tooltip_text(Some(&gettext("Places")));
        places_button.set_popover(Some(&places_popover));
        let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh.set_tooltip_text(Some(&gettext("Refresh")));
        refresh.set_action_name(Some("win.refresh"));
        let units_menu = gio::Menu::new();
        units_menu.append(Some(&gettext("Celsius")), Some("win.units::metric"));
        units_menu.append(Some(&gettext("Fahrenheit")), Some("win.units::imperial"));
        let menu = gio::Menu::new();
        menu.append_section(None, &units_menu);
        menu.append(Some(&gettext("About Weather")), Some("app.about"));
        let header = AsenaHeaderBarBuilder::new()
            .title_widget(title.clone())
            .pack_start(&places_button)
            .pack_end(&refresh)
            .primary_menu(menu.upcast_ref())
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Weather"))
            .default_size(500, 680)
            .min_size(360, 480)
            .content(&toolbar)
            .build();

        let state = Rc::new(State {
            window,
            saved: RefCell::default(),
            stack,
            title,
            places,
            places_popover,
            error,
            icon,
            temperature,
            summary,
            feels_like,
            hours,
            days,
            humidity,
            wind,
            pressure,
            sunrise,
            sunset,
            updated,
            generation: Cell::new(0),
            shown: Cell::new(false),
        });
        let this = Self { state };
        this.install_actions();
        let cloned = this.clone();
        this.state.places.connect_row_activated(move |_, row| {
            if let Ok(index) = row.widget_name().parse::<usize>() {
                cloned.state.places_popover.popdown();
                cloned.state.saved.borrow_mut().selected = index;
                cloned.save();
                cloned.show_place(false);
            }
        });
        this.load();
        // Refresh while open; the service updates its model every 15 minutes.
        let weak = Rc::downgrade(&this.state);
        glib::timeout_add_seconds_local(30 * 60, move || {
            let Some(state) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            Self { state }.show_place(true);
            glib::ControlFlow::Continue
        });
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let action = gio::SimpleAction::new("add-place", None);
        let this = self.clone();
        action.connect_activate(move |_, _| this.choose_place());
        window.add_action(&action);
        let action = gio::SimpleAction::new("refresh", None);
        let this = self.clone();
        action.connect_activate(move |_, _| this.show_place(true));
        window.add_action(&action);
        let units = gio::SimpleAction::new_stateful(
            "units",
            Some(glib::VariantTy::STRING),
            &Units::Metric.id().to_variant(),
        );
        let this = self.clone();
        units.connect_activate(move |action, parameter| {
            let Some(id) = parameter.and_then(|value| value.str().map(str::to_owned)) else {
                return;
            };
            action.set_state(&id.to_variant());
            let units = Units::from_id(&id);
            if this.state.saved.borrow().units != units {
                this.state.saved.borrow_mut().units = units;
                this.save();
                this.show_place(false);
            }
        });
        window.add_action(&units);
    }

    fn load(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let text =
                gio::spawn_blocking(|| std::fs::read_to_string(saved_path()).unwrap_or_default())
                    .await
                    .unwrap_or_default();
            let saved = Saved::parse(&text);
            if let Some(action) = this.state.window.lookup_action("units") {
                action.change_state(&saved.units.id().to_variant());
            }
            this.state.saved.replace(saved);
            this.show_place(false);
        });
    }

    fn save(&self) {
        let Ok(text) = serde_json::to_string_pretty(&*self.state.saved.borrow()) else {
            return;
        };
        glib::spawn_future_local(async move {
            let result =
                gio::spawn_blocking(move || storage::write_atomic(&saved_path(), text.as_bytes()))
                    .await;
            if let Ok(Err(error)) = result {
                tracing::warn!(%error, "Could not save the weather places");
            }
        });
    }

    fn render_places(&self) {
        let list = &self.state.places;
        list.remove_all();
        let saved = self.state.saved.borrow();
        for (index, place) in saved.places.iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&place.name))
                .subtitle(glib::markup_escape_text(&place.region))
                .activatable(true)
                .build();
            row.set_widget_name(&index.to_string());
            if index == saved.selected {
                row.add_prefix(&gtk::Image::from_icon_name("object-select-symbolic"));
            }
            let remove = gtk::Button::from_icon_name("user-trash-symbolic");
            remove.add_css_class("flat");
            remove.set_valign(gtk::Align::Center);
            remove.set_tooltip_text(Some(&gettext("Remove")));
            let this = self.clone();
            remove.connect_clicked(move |_| {
                this.state.saved.borrow_mut().remove(index);
                this.save();
                this.show_place(false);
            });
            row.add_suffix(&remove);
            list.append(&row);
        }
        list.set_visible(!saved.places.is_empty());
    }

    /// Shows the selected place: the cached forecast first, then a fresh one
    /// when the cache is old or `force` is set.
    fn show_place(&self, force: bool) {
        self.render_places();
        let generation = self.state.generation.get() + 1;
        self.state.generation.set(generation);
        let (place, units) = {
            let saved = self.state.saved.borrow();
            (saved.current().cloned(), saved.units)
        };
        let Some(place) = place else {
            self.state.title.set_title(&gettext("Weather"));
            self.state.title.set_subtitle("");
            self.state.shown.set(false);
            self.state.stack.set_visible_child_name("empty");
            return;
        };
        self.state.title.set_title(&place.name);
        self.state.title.set_subtitle(&place.region);
        let this = self.clone();
        glib::spawn_future_local(async move {
            let path = cache_path(&place, units);
            let cached = {
                let path = path.clone();
                gio::spawn_blocking(move || {
                    let text = std::fs::read_to_string(&path).ok()?;
                    let modified = std::fs::metadata(&path)
                        .and_then(|metadata| metadata.modified())
                        .ok()?
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()?
                        .as_secs() as i64;
                    Some((text, modified))
                })
                .await
                .ok()
                .flatten()
            };
            let mut fresh = false;
            this.state.shown.set(false);
            if let Some((text, modified)) = cached {
                if let Ok(forecast) = forecast::parse(&text) {
                    this.render(&forecast, units, modified);
                    fresh = now_unix() - modified < FRESH_SECONDS;
                }
            }
            if fresh && !force {
                return;
            }
            if !this.state.shown.get() {
                this.state.stack.set_visible_child_name("loading");
            }
            let url = forecast::forecast_url(place.latitude, place.longitude, units);
            let result = fetch(&url).await.and_then(|text| {
                forecast::parse(&text)
                    .map(|forecast| (text, forecast))
                    .map_err(|error| error.to_string())
            });
            if this.state.generation.get() != generation {
                return;
            }
            match result {
                Ok((text, forecast)) => {
                    this.render(&forecast, units, now_unix());
                    let saved =
                        gio::spawn_blocking(move || storage::write_atomic(&path, text.as_bytes()))
                            .await;
                    if let Ok(Err(error)) = saved {
                        tracing::warn!(%error, "Could not cache the forecast");
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "Could not load the forecast");
                    if this.state.shown.get() {
                        this.state
                            .window
                            .show_toast(&gettext("Could not refresh the forecast"));
                    } else {
                        this.state.error.set_description(Some(&gettext(
                            "Check the internet connection and try again.",
                        )));
                        this.state.stack.set_visible_child_name("error");
                    }
                }
            }
        });
    }

    fn render(&self, forecast: &Forecast, units: Units, fetched: i64) {
        let state = &self.state;
        let current = &forecast.current;
        let (text, icon) = forecast::condition(current.code, current.is_day);
        state.icon.set_icon_name(Some(icon));
        state
            .temperature
            .set_text(&forecast::temperature(current.temperature));
        state.summary.set_text(&text);
        let mut feels = gettext("Feels like {temperature}")
            .replace("{temperature}", &forecast::temperature(current.feels_like));
        if let Some(today) = forecast.days.first() {
            feels.push_str(&format!(
                "  ·  {} / {}",
                forecast::temperature(today.min),
                forecast::temperature(today.max)
            ));
        }
        state.feels_like.set_text(&feels);

        while let Some(child) = state.hours.first_child() {
            state.hours.remove(&child);
        }
        for (index, hour) in forecast.hours.iter().enumerate() {
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 6);
            cell.set_margin_top(12);
            cell.set_margin_bottom(12);
            cell.set_margin_start(6);
            cell.set_margin_end(6);
            cell.set_size_request(56, -1);
            let time = if index == 0 {
                gettext("Now")
            } else {
                hour.time.clone()
            };
            let time = dim(&time);
            time.add_css_class("caption");
            let (description, icon) = forecast::condition(hour.code, hour.is_day);
            let image = gtk::Image::from_icon_name(icon);
            image.set_pixel_size(24);
            image.set_tooltip_text(Some(&description));
            let value = gtk::Label::new(Some(&forecast::temperature(hour.temperature)));
            value.add_css_class("numeric");
            cell.append(&time);
            cell.append(&image);
            cell.append(&value);
            let rain = dim(&hour
                .precipitation
                .filter(|chance| *chance >= 10.0)
                .map(|chance| format!("{}%", chance.round() as i64))
                .unwrap_or_default());
            rain.add_css_class("caption");
            rain.add_css_class("numeric");
            cell.append(&rain);
            state.hours.append(&cell);
        }

        state.days.remove_all();
        for (index, day) in forecast.days.iter().enumerate() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.set_margin_top(10);
            row.set_margin_bottom(10);
            row.set_margin_start(12);
            row.set_margin_end(12);
            let name = gtk::Label::new(Some(&if index == 0 {
                gettext("Today")
            } else {
                weekday(&day.date)
            }));
            name.set_xalign(0.0);
            name.set_hexpand(true);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let (description, icon) = forecast::condition(day.code, true);
            let image = gtk::Image::from_icon_name(icon);
            image.set_tooltip_text(Some(&description));
            let rain = dim(&day
                .precipitation
                .filter(|chance| *chance >= 10.0)
                .map(|chance| format!("{}%", chance.round() as i64))
                .unwrap_or_default());
            rain.add_css_class("numeric");
            rain.set_width_chars(4);
            rain.set_xalign(1.0);
            let min = dim(&forecast::temperature(day.min));
            min.add_css_class("numeric");
            min.set_width_chars(4);
            min.set_xalign(1.0);
            let max = gtk::Label::new(Some(&forecast::temperature(day.max)));
            max.add_css_class("numeric");
            max.set_width_chars(4);
            max.set_xalign(1.0);
            row.append(&name);
            row.append(&rain);
            row.append(&image);
            row.append(&min);
            row.append(&max);
            state.days.append(&row);
        }

        state
            .humidity
            .set_text(&format!("{}%", current.humidity.round() as i64));
        state.wind.set_text(&format!(
            "{} {}",
            units.wind(current.wind_speed),
            forecast::compass(current.wind_direction)
        ));
        state.pressure.set_text(
            &gettext("{pressure} hPa")
                .replace("{pressure}", &(current.pressure.round() as i64).to_string()),
        );
        let today = forecast.days.first();
        state.sunrise.set_text(
            today
                .and_then(|day| day.sunrise.as_deref())
                .unwrap_or_default(),
        );
        state.sunset.set_text(
            today
                .and_then(|day| day.sunset.as_deref())
                .unwrap_or_default(),
        );
        state
            .updated
            .set_text(&gettext("Updated {time}").replace("{time}", &time_of(fetched)));
        state.stack.set_visible_child_name("forecast");
        state.shown.set(true);
    }

    fn choose_place(&self) {
        self.state.places_popover.popdown();
        let dialog = adw::Dialog::new();
        dialog.set_title(&gettext("Add place"));
        dialog.set_content_width(420);
        dialog.set_content_height(520);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search for a city")));
        search.set_margin_start(12);
        search.set_margin_end(12);
        search.set_margin_bottom(8);
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        let status = adw::StatusPage::builder()
            .icon_name("edit-find-symbolic")
            .build();
        status.add_css_class("compact");
        let stack = gtk::Stack::new();
        stack.add_named(&status, Some("status"));
        stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("results"),
        );
        let results: Rc<RefCell<Vec<Place>>> = Rc::default();
        let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
        let query_generation = Rc::new(Cell::new(0u64));
        let language = places::language(
            glib::language_names()
                .first()
                .map(|name| name.as_str())
                .unwrap_or("en"),
        );
        {
            let list = list.clone();
            let stack = stack.clone();
            let status = status.clone();
            let results = results.clone();
            search.connect_search_changed(move |entry| {
                if let Some(source) = pending.borrow_mut().take() {
                    source.remove();
                }
                let query = entry.text().trim().to_owned();
                let generation = query_generation.get() + 1;
                query_generation.set(generation);
                if query.chars().count() < 2 {
                    status.set_title("");
                    stack.set_visible_child_name("status");
                    return;
                }
                let list = list.clone();
                let stack = stack.clone();
                let status = status.clone();
                let results = results.clone();
                let language = language.clone();
                let query_generation = query_generation.clone();
                let slot = pending.clone();
                let source = glib::timeout_add_local_once(Duration::from_millis(400), move || {
                    slot.borrow_mut().take();
                    glib::spawn_future_local(async move {
                        let found = fetch(&places::search_url(&query, &language))
                            .await
                            .and_then(|text| {
                                places::parse_search(&text).map_err(|error| error.to_string())
                            });
                        if query_generation.get() != generation {
                            return;
                        }
                        list.remove_all();
                        match found {
                            Ok(found) if !found.is_empty() => {
                                for (index, place) in found.iter().enumerate() {
                                    let row = adw::ActionRow::builder()
                                        .title(glib::markup_escape_text(&place.name))
                                        .subtitle(glib::markup_escape_text(&place.region))
                                        .activatable(true)
                                        .build();
                                    row.set_widget_name(&index.to_string());
                                    list.append(&row);
                                }
                                results.replace(found);
                                stack.set_visible_child_name("results");
                            }
                            Ok(_) => {
                                status.set_title(&gettext("No results"));
                                stack.set_visible_child_name("status");
                            }
                            Err(error) => {
                                tracing::warn!(%error, "Place search failed");
                                status.set_title(&gettext("Could not search"));
                                stack.set_visible_child_name("status");
                            }
                        }
                    });
                });
                pending.replace(Some(source));
            });
        }
        let this = self.clone();
        let weak = dialog.downgrade();
        list.connect_row_activated(move |_, row| {
            let index = row.widget_name().parse::<usize>().ok();
            let place = index.and_then(|index| results.borrow().get(index).cloned());
            if let Some(place) = place {
                this.state.saved.borrow_mut().add(place);
                this.save();
                this.show_place(false);
            }
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        });
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.add_top_bar(&search);
        toolbar.set_content(Some(&stack));
        dialog.set_child(Some(&toolbar));
        dialog.present(Some(self.state.window.window()));
        search.grab_focus();
    }
}
