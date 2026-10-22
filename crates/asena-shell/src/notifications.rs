use crate::surface::Surface;
use asena_core::{i18n::gettext, notification::Notification};
use asena_services::notifications::NotificationModel;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
    time::{Duration, Instant},
};

struct Popup {
    window: gtk::ApplicationWindow,
    content: gtk::Box,
    timer: RefCell<Option<glib::SourceId>>,
    remaining: Cell<Option<Duration>>,
    started: Cell<Instant>,
    hovered: Cell<bool>,
}

impl Drop for Popup {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.remove();
        }
        self.window.close();
    }
}

#[derive(Clone)]
pub struct Notifications {
    surface: Surface,
    list: gtk::Box,
    empty_state: gtk::Box,
    calendar: gtk::Calendar,
    clock_time: gtk::Label,
    clock_date: gtk::Label,
    badge: gtk::Label,
    clear_btn: gtk::Button,
    state: Rc<State>,
}

struct State {
    suppressed: Cell<bool>,
    app: adw::Application,
    model: NotificationModel,
    settings: gio::Settings,
    popups: RefCell<BTreeMap<u32, Rc<Popup>>>,
    pending: RefCell<VecDeque<u32>>,
    timestamps: RefCell<BTreeMap<u32, glib::DateTime>>,
}

impl Notifications {
    pub fn new(
        app: &adw::Application,
        model: &NotificationModel,
        mpris: &asena_services::mpris::MprisModel,
        settings: &gio::Settings,
    ) -> Self {
        model.set_blocked(
            settings
                .strv("blocked-applications")
                .iter()
                .map(ToString::to_string),
        );
        let blocked = model.clone();
        settings.connect_changed(Some("blocked-applications"), move |settings, _| {
            blocked.set_blocked(
                settings
                    .strv("blocked-applications")
                    .iter()
                    .map(ToString::to_string),
            )
        });

        // A full height side panel: date, calendar, media and notifications.
        let surface = Surface::sidebar(app, "asena-notification-center");
        let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
        body.add_css_class("asena-sidebar");
        body.set_vexpand(true);

        let date = gtk::Box::new(gtk::Orientation::Vertical, 2);
        date.add_css_class("nc-clock-card");
        let clock_time = gtk::Label::new(None);
        clock_time.add_css_class("nc-clock-time");
        clock_time.set_xalign(0.0);
        date.append(&clock_time);
        let clock_date = gtk::Label::new(None);
        clock_date.add_css_class("nc-clock-date");
        clock_date.set_xalign(0.0);
        date.append(&clock_date);
        body.append(&date);

        let calendar = gtk::Calendar::new();
        calendar.set_show_day_names(true);
        calendar.set_show_heading(true);
        calendar.add_css_class("nc-calendar-card");
        body.append(&calendar);

        let media = crate::media::create(mpris);
        media.add_css_class("asena-sidebar-media");
        body.append(&media);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.add_css_class("nc-header");
        let title = gtk::Label::new(Some(&gettext("Notifications")));
        title.add_css_class("nc-title");
        title.set_xalign(0.0);
        header.append(&title);
        let badge = gtk::Label::new(None);
        badge.add_css_class("nc-badge");
        badge.add_css_class("numeric");
        badge.set_visible(false);
        header.append(&badge);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
        let clear_btn = gtk::Button::with_label(&gettext("Clear all"));
        clear_btn.add_css_class("flat");
        clear_btn.add_css_class("nc-text-btn");
        let cloned_model = model.clone();
        clear_btn.connect_clicked(move |_| cloned_model.clear());
        header.append(&clear_btn);
        let dnd_btn = gtk::Button::from_icon_name("notifications-disabled-symbolic");
        dnd_btn.add_css_class("asena-icon-button");
        dnd_btn.set_tooltip_text(Some(&gettext("Do not disturb")));
        let weak_dnd = dnd_btn.downgrade();
        let update_dnd_style = move |s: &gio::Settings| {
            if let Some(btn) = weak_dnd.upgrade() {
                if s.boolean("do-not-disturb") {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
        };
        update_dnd_style(settings);
        settings.connect_changed(Some("do-not-disturb"), move |s, _| update_dnd_style(s));
        let dnd_settings = settings.clone();
        dnd_btn.connect_clicked(move |_| {
            let current = dnd_settings.boolean("do-not-disturb");
            let _ = dnd_settings.set_boolean("do-not-disturb", !current);
        });
        header.append(&dnd_btn);
        body.append(&header);

        let scroll_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        scroll_content.append(&list);
        let empty_state = gtk::Box::new(gtk::Orientation::Vertical, 8);
        empty_state.add_css_class("asena-empty-state");
        empty_state.set_halign(gtk::Align::Center);
        empty_state.set_valign(gtk::Align::Center);
        empty_state.set_vexpand(true);
        let empty_icon = gtk::Image::from_icon_name("notifications-symbolic");
        empty_icon.set_pixel_size(40);
        empty_icon.add_css_class("asena-empty-state-icon");
        empty_state.append(&empty_icon);
        let empty_title = gtk::Label::new(Some(&gettext("No new notifications")));
        empty_title.add_css_class("asena-empty-state-title");
        empty_state.append(&empty_title);
        scroll_content.append(&empty_state);
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&scroll_content));
        body.append(&scroll);

        surface.content.append(&body);

        let state = Rc::new(State {
            suppressed: Cell::new(false),
            app: app.clone(),
            model: model.clone(),
            settings: settings.clone(),
            popups: RefCell::default(),
            pending: RefCell::default(),
            timestamps: RefCell::default(),
        });

        let controller = Self {
            surface,
            list,
            empty_state,
            calendar,
            clock_time,
            clock_date,
            badge,
            clear_btn,
            state,
        };

        // The date heading refreshes when the center opens; no idle timer.
        controller.update_clock();

        let cloned = controller.clone();
        model.connect_received(move |_, id| cloned.received(id));

        let cloned = controller.clone();
        model.connect_changed(move |_| {
            cloned.sync();
            cloned.refresh();
        });

        let cloned = controller.clone();
        settings.connect_changed(Some("do-not-disturb"), move |_, _| cloned.sync());

        let cloned = controller.clone();
        controller
            .surface
            .window
            .connect_visible_notify(move |window| {
                if window.is_visible() {
                    cloned.state.model.mark_read();
                    cloned.update_clock();
                }
                cloned.sync();
            });

        controller.refresh();
        controller
    }

    fn update_clock(&self) {
        if let Ok(now) = glib::DateTime::now_local() {
            if let Ok(day) = now.format("%A") {
                self.clock_time.set_text(&day);
            }
            if let Ok(date) = now.format("%e %B %Y") {
                self.clock_date.set_text(date.trim());
            }
            self.calendar.select_day(&now);
        }
    }

    pub fn toggle(&self) {
        self.update_clock();
        if self.surface.toggle(380, 0) {
            self.state.model.mark_read();
            self.refresh();
        }
    }

    pub fn hide(&self) {
        self.surface.hide();
    }

    pub fn suppress_popups(&self, suppressed: bool) {
        self.state.suppressed.set(suppressed);
        self.sync();
    }

    fn refresh(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        let mut items = self.state.model.items();
        let count = items.len();

        if count > 0 {
            self.badge.set_visible(true);
            self.badge.set_text(&count.to_string());
            self.clear_btn.set_sensitive(true);
            self.empty_state.set_visible(false);
            self.list.set_visible(true);
        } else {
            self.badge.set_visible(false);
            self.clear_btn.set_sensitive(false);
            self.empty_state.set_visible(true);
            self.list.set_visible(false);
        }

        // App grouping
        let mut apps = Vec::new();
        for item in &items {
            if !apps.contains(&item.app) {
                apps.push(item.app.clone());
            }
        }
        items.sort_by_key(|item| apps.iter().position(|app| app == &item.app));

        for item in items {
            let time_str = self
                .state
                .timestamps
                .borrow()
                .get(&item.id)
                .and_then(|dt| dt.format("%H:%M").ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| gettext("Just now"));

            let card = notification_card(&item, &self.state.model, &time_str);
            self.list.append(&card);
        }
    }

    fn received(&self, id: u32) {
        if let Ok(now) = glib::DateTime::now_local() {
            self.state.timestamps.borrow_mut().insert(id, now);
        }

        let Some(item) = self.state.model.active(id) else {
            return;
        };

        if let Some(popup) = self.state.popups.borrow().get(&id).cloned() {
            self.update_popup(&popup, &item);
            return;
        }

        if self.state.settings.boolean("do-not-disturb") && !item.critical {
            return;
        }

        if !self.state.pending.borrow().contains(&id) {
            self.state.pending.borrow_mut().push_back(id);
        }

        self.sync();
    }

    pub fn sync(&self) {
        let closed: Vec<_> = self
            .state
            .popups
            .borrow()
            .keys()
            .copied()
            .filter(|id| self.state.model.active(*id).is_none())
            .collect();

        for id in closed {
            self.state.popups.borrow_mut().remove(&id);
        }

        loop {
            if self.state.popups.borrow().len() >= 3 {
                break;
            }
            let id = self.state.pending.borrow_mut().pop_front();
            let Some(id) = id else {
                break;
            };
            let Some(item) = self.state.model.active(id) else {
                continue;
            };
            if self.state.settings.boolean("do-not-disturb") && !item.critical {
                continue;
            }
            let popup = self.create_popup(&item);
            self.state.popups.borrow_mut().insert(id, popup);
        }

        let layout = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA)
            .map(|settings| asena_core::layout::PanelLayout::from_settings(&settings))
            .unwrap_or_default();
        let top = layout.edge == asena_core::layout::PanelEdge::Top;
        let left = layout.edge == asena_core::layout::PanelEdge::Left;
        let vertical = if top { Edge::Top } else { Edge::Bottom };
        let side = if left { Edge::Left } else { Edge::Right };
        let mut margin = if layout.is_horizontal() {
            layout.thickness + 16
        } else {
            16
        };

        for popup in self.state.popups.borrow().values().rev() {
            popup
                .window
                .set_visible(!self.surface.window.is_visible() && !self.state.suppressed.get());

            for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                popup.window.set_anchor(edge, false);
                popup.window.set_margin(edge, 0);
            }
            popup.window.set_anchor(vertical, true);
            popup.window.set_anchor(side, true);
            popup.window.set_margin(
                side,
                if layout.is_horizontal() {
                    16
                } else {
                    layout.thickness + 16
                },
            );
            popup.window.set_margin(vertical, margin);
            margin += popup.window.height().max(100) + 8;
        }
    }

    fn create_popup(&self, item: &Notification) -> Rc<Popup> {
        let window = gtk::ApplicationWindow::builder()
            .application(&self.state.app)
            .decorated(false)
            .default_width(340)
            .build();
        window.init_layer_shell();
        window.set_namespace(Some("asena-notification"));
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_exclusive_zone(-1);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Right, 16);
        window.add_css_class("asena-surface");

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.set_child(Some(&content));

        let popup = Rc::new(Popup {
            window,
            content,
            timer: RefCell::default(),
            remaining: Cell::new(None),
            started: Cell::new(Instant::now()),
            hovered: Cell::new(false),
        });

        let motion = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(&popup);
        motion.connect_enter(move |_, _, _| {
            if let Some(popup) = weak.upgrade() {
                popup.hovered.set(true);
                if let Some(timer) = popup.timer.borrow_mut().take() {
                    timer.remove();
                    if let Some(remaining) = popup.remaining.get() {
                        popup.remaining.set(Some(
                            remaining.saturating_sub(popup.started.get().elapsed()),
                        ));
                    }
                }
            }
        });

        let weak = Rc::downgrade(&popup);
        let model = self.state.model.clone();
        let id = item.id;
        motion.connect_leave(move |_| {
            if let Some(popup) = weak.upgrade() {
                popup.hovered.set(false);
                schedule(&popup, &model, id);
            }
        });
        popup.window.add_controller(motion);

        self.update_popup(&popup, item);
        popup.window.present();
        popup
    }

    fn update_popup(&self, popup: &Rc<Popup>, item: &Notification) {
        if let Some(timer) = popup.timer.borrow_mut().take() {
            timer.remove();
        }
        while let Some(child) = popup.content.first_child() {
            popup.content.remove(&child);
        }

        let time_str = gettext("Just now");
        popup
            .content
            .append(&notification_card(item, &self.state.model, &time_str));
        popup.remaining.set(
            item.timeout_ms
                .map(|value| Duration::from_millis(value.into())),
        );
        schedule(popup, &self.state.model, item.id);
    }
}

fn schedule(popup: &Rc<Popup>, model: &NotificationModel, id: u32) {
    if popup.hovered.get() {
        return;
    }
    let Some(remaining) = popup.remaining.get() else {
        return;
    };
    popup.started.set(Instant::now());
    let weak = Rc::downgrade(popup);
    let model = model.clone();
    popup
        .timer
        .replace(Some(glib::timeout_add_local_once(remaining, move || {
            if let Some(popup) = weak.upgrade() {
                popup.timer.borrow_mut().take();
            }
            model.close(id, 1);
        })));
}

fn notification_card(item: &Notification, model: &NotificationModel, time_str: &str) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.add_css_class("asena-notification-card");

    // Header: App icon + App name + Timestamp + Dismiss button
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("notification-header");

    let icon = gtk::Image::from_icon_name("dialog-information-symbolic");
    icon.set_pixel_size(20);
    if let Some(image) = &item.image {
        let bytes = glib::Bytes::from_owned(image.rgba.clone());
        let texture = gtk::gdk::MemoryTexture::new(
            image.width,
            image.height,
            gtk::gdk::MemoryFormat::R8g8b8a8,
            &bytes,
            image.width as usize * 4,
        );
        icon.set_paintable(Some(&texture));
    } else if !item.icon.contains('/') && !item.icon.contains(':') && !item.icon.is_empty() {
        icon.set_icon_name(Some(&item.icon));
    } else if !item.icon.is_empty() {
        load_local_image(&icon, &item.icon);
    }
    header.append(&icon);

    let app = gtk::Label::new(Some(&item.app));
    app.set_xalign(0.0);
    app.add_css_class("notification-app-name");
    app.set_ellipsize(gtk::pango::EllipsizeMode::End);
    header.append(&app);

    let time_lbl = gtk::Label::new(Some(time_str));
    time_lbl.set_xalign(0.0);
    time_lbl.set_hexpand(true);
    time_lbl.add_css_class("notification-time");
    header.append(&time_lbl);

    let close = gtk::Button::from_icon_name("window-close-symbolic");
    close.add_css_class("notification-close-btn");
    let dismiss_str = gettext("Dismiss");
    close.set_tooltip_text(Some(&dismiss_str));
    let cloned = model.clone();
    let id = item.id;
    close.connect_clicked(move |_| cloned.close(id, 2));
    header.append(&close);

    content.append(&header);

    // Summary
    let summary = gtk::Label::new(Some(&item.summary));
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    summary.set_lines(2);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    summary.add_css_class("notification-summary");
    content.append(&summary);

    // Body
    if !item.body.is_empty() {
        let body = gtk::Label::new(None);
        body.set_markup(&item.body);
        body.set_xalign(0.0);
        body.set_wrap(true);
        body.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        body.set_max_width_chars(38);
        body.set_lines(3);
        body.set_ellipsize(gtk::pango::EllipsizeMode::End);
        body.add_css_class("notification-body");
        body.connect_activate_link(|_, uri| {
            if asena_core::notification::safe_uri(uri) {
                let launcher = gtk::UriLauncher::new(uri);
                glib::spawn_future_local(async move {
                    if let Err(error) = launcher.launch_future(None::<&gtk::Window>).await {
                        tracing::warn!(%error, "Could not open notification link");
                    }
                });
            }
            glib::Propagation::Stop
        });
        content.append(&body);
    }

    // Actions
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let live = model.active(item.id).is_some();
    for (key, label) in &item.actions {
        if key == "default" {
            continue;
        }
        let button = gtk::Button::with_label(label);
        button.add_css_class("notification-action-btn");
        button.set_sensitive(live);
        let key = key.clone();
        let model = model.clone();
        button.connect_clicked(move |_| model.invoke(id, &key, None));
        actions.append(&button);
    }
    if actions.first_child().is_some() {
        content.append(&actions);
    }

    if live && item.actions.iter().any(|(key, _)| key == "default") {
        let click = gtk::GestureClick::new();
        let model = model.clone();
        click.connect_released(move |_, _, _, _| model.invoke(id, "default", None));
        summary.add_controller(click);
    }

    content
}

pub(crate) fn load_local_image(icon: &gtk::Image, source: &str) {
    let path = if source.starts_with("file:") {
        gio::File::for_uri(source).path()
    } else {
        let path = std::path::PathBuf::from(source);
        path.is_absolute().then_some(path)
    };
    let Some(path) = path else {
        return;
    };
    let weak = icon.downgrade();
    glib::spawn_future_local(async move {
        let image = gio::spawn_blocking(move || {
            if std::fs::metadata(&path).ok()?.len() > 16_777_216 {
                return None;
            }
            let (_, width, height) = gtk::gdk_pixbuf::Pixbuf::file_info(&path)?;
            if !(1..=2048).contains(&width) || !(1..=2048).contains(&height) {
                return None;
            }
            let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file(path).ok()?;
            asena_core::notification::image_data(
                pixbuf.width(),
                pixbuf.height(),
                pixbuf.rowstride(),
                pixbuf.has_alpha(),
                pixbuf.bits_per_sample(),
                pixbuf.n_channels(),
                pixbuf.read_pixel_bytes().as_ref(),
            )
        })
        .await;
        if let (Ok(Some(image)), Some(icon)) = (image, weak.upgrade()) {
            let bytes = glib::Bytes::from_owned(image.rgba);
            let texture = gtk::gdk::MemoryTexture::new(
                image.width,
                image.height,
                gtk::gdk::MemoryFormat::R8g8b8a8,
                &bytes,
                image.width as usize * 4,
            );
            icon.set_paintable(Some(&texture));
        }
    });
}
