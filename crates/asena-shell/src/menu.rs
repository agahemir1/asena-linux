//! Start menu with three layouts sharing one index, search, pins and usage.
//!
//! * Odak (compact): a single list. Pinned applications first, then every
//!   application under letter headers. A letter header opens a letter grid
//!   inside the menu to jump between sections without narrowing the list.
//! * Classic: categories on the left, the selected group on the right.
//! * Grid: all applications as large tiles.
//!
//! The search field always sits on the edge that faces the panel, so the
//! pointer and the eye stay where the menu was opened.
use crate::{
    search::{self, SearchProvider},
    surface::Surface,
};
use asena_core::{
    apps::DesktopEntry,
    i18n::gettext,
    launcher::{self, Category, Layout},
    layout::{PanelEdge, PanelLayout},
    usage::Usage,
};
use asena_services::app_catalog::AppCatalog;
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

#[derive(Clone)]
pub struct Menu {
    surface: Surface,
    state: Rc<State>,
}

#[derive(Default)]
struct Views {
    search: Option<gtk::SearchEntry>,
    stack: Option<gtk::Stack>,
    results: Option<gtk::ListBox>,
    empty: Option<gtk::Box>,
    /// Compact layout.
    list: Option<gtk::ListBox>,
    list_scroll: Option<gtk::ScrolledWindow>,
    picker: Option<gtk::Box>,
    picker_grid: Option<gtk::FlowBox>,
    letters: Vec<(String, gtk::ListBoxRow)>,
    /// Classic layout.
    categories: Option<gtk::ListBox>,
    group: Option<gtk::ListBox>,
    /// Grid layout.
    grid: Option<gtk::FlowBox>,
}

struct State {
    apps: AppCatalog,
    settings: gio::Settings,
    user: asena_services::user::UserModel,
    daemon: asena_services::property_model::PropertyModel,
    layout: Cell<Layout>,
    category: RefCell<String>,
    views: RefCell<Views>,
    generation: Cell<u64>,
    actions: RefCell<Vec<search::Action>>,
    usage: RefCell<Usage>,
    pending: RefCell<Option<glib::JoinHandle<()>>>,
    saves: async_channel::Sender<Usage>,
}

fn collate(entries: &mut [DesktopEntry]) {
    entries.sort_by(|a, b| {
        glib::CollationKey::from(a.name.as_str()).cmp(&glib::CollationKey::from(b.name.as_str()))
    });
}

fn icon_for(entry_icon: Option<&str>, size: i32) -> gtk::Image {
    let image = gtk::Image::new();
    image.set_pixel_size(size);
    match entry_icon.and_then(|icon| gio::Icon::for_string(icon).ok()) {
        Some(icon) => image.set_from_gicon(&icon),
        None => image.set_icon_name(Some("org.asenalinux.App")),
    }
    image
}

fn section_row(title: &str) -> gtk::ListBoxRow {
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("asena-menu-section");
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);
    row.set_focusable(false);
    row.set_child(Some(&label));
    row
}

fn app_row(entry: &DesktopEntry, icon_size: i32) -> gtk::ListBoxRow {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&icon_for(entry.icon.as_deref(), icon_size));
    let name = gtk::Label::new(Some(&entry.name));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    content.append(&name);
    let row = gtk::ListBoxRow::new();
    row.add_css_class("asena-menu-app");
    row.set_child(Some(&content));
    row.set_widget_name(&entry.id);
    row.set_tooltip_text(entry.generic_name.as_deref());
    row
}

fn launch_settings(panel: Option<&str>) {
    match panel {
        Some(panel) => crate::launch::command(&format!("asena-settings {panel}")),
        None => crate::launch::command("asena-settings"),
    }
}

impl Menu {
    pub fn new(
        app: &adw::Application,
        apps: &AppCatalog,
        settings: &gio::Settings,
        user: &asena_services::user::UserModel,
        daemon: &asena_services::property_model::PropertyModel,
    ) -> Self {
        let surface = Surface::new(app, "asena-menu", Default::default(), false);
        let (saves, receiver) = async_channel::unbounded();
        let path = std::env::var_os("XDG_STATE_HOME")
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| glib::home_dir().join(".local/state"))
            .join("asena/usage.json");
        let load_path = path.clone();
        glib::spawn_future_local(async move {
            while let Ok(usage) = receiver.recv().await {
                let path = path.clone();
                match gio::spawn_blocking(move || asena_core::usage::save(&path, &usage)).await {
                    Ok(Ok(())) => {}
                    error => tracing::warn!(?error, "Could not persist application usage"),
                }
            }
        });
        let state = Rc::new(State {
            apps: apps.clone(),
            settings: settings.clone(),
            user: user.clone(),
            daemon: daemon.clone(),
            layout: Cell::new(Layout::from_key(&settings.string("menu-layout"))),
            category: RefCell::new("all".into()),
            views: RefCell::default(),
            generation: Cell::new(0),
            actions: RefCell::default(),
            usage: RefCell::default(),
            pending: RefCell::default(),
            saves,
        });
        let menu = Self { surface, state };
        let weak = Rc::downgrade(&menu.state);
        glib::spawn_future_local(async move {
            if let Ok(loaded) =
                gio::spawn_blocking(move || asena_core::usage::load(&load_path)).await
            {
                if let Some(state) = weak.upgrade() {
                    let mut usage = state.usage.borrow_mut();
                    for (id, count) in loaded {
                        let value = usage.entry(id).or_default();
                        *value = value.saturating_add(count);
                    }
                }
            }
        });
        menu.build();
        let cloned = menu.clone();
        apps.connect_changed(move |_| cloned.refresh());
        let cloned = menu.clone();
        settings.connect_changed(None, move |settings, key| match key {
            "menu-pinned-apps" => cloned.refresh(),
            "menu-layout" => {
                cloned
                    .state
                    .layout
                    .set(Layout::from_key(&settings.string("menu-layout")));
                cloned.build();
            }
            "panel-edge" => cloned.build(),
            _ => {}
        });
        menu
    }

    fn size(&self) -> (i32, i32) {
        match self.state.layout.get() {
            Layout::Compact => (380, 560),
            Layout::Classic => (580, 520),
            Layout::Grid => (660, 580),
        }
    }

    pub fn toggle(&self) {
        let start = Instant::now();
        let (width, height) = self.size();
        if self.surface.toggle(width, height) {
            let search = self.state.views.borrow().search.clone();
            if let Some(search) = search {
                search.set_text("");
                search.grab_focus();
            }
            self.refresh();
        }
        tracing::debug!(
            elapsed_us = start.elapsed().as_micros(),
            visible = self.surface.window.is_visible(),
            "Menu visibility requested"
        );
    }

    pub fn hide(&self) {
        self.surface.hide();
    }

    pub fn connect_visibility(&self, callback: impl Fn(bool) + 'static) {
        self.surface
            .window
            .connect_visible_notify(move |window| callback(window.is_visible()));
    }

    /// Rebuilds the surface for the current layout and panel edge.
    fn build(&self) {
        let content = &self.surface.content;
        while let Some(child) = content.first_child() {
            content.remove(&child);
        }
        let layout = self.state.layout.get();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.add_css_class("asena-menu");
        root.add_css_class(match layout {
            Layout::Compact => "compact",
            Layout::Classic => "classic",
            Layout::Grid => "grid",
        });

        let search = gtk::SearchEntry::new();
        search.add_css_class("asena-menu-search");
        search.set_placeholder_text(Some(&gettext("Search applications and settings")));
        search.set_key_capture_widget(Some(&self.surface.window));
        search.set_search_delay(0);
        search.set_hexpand(true);

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.set_transition_duration(120);

        let mut views = Views {
            search: Some(search.clone()),
            stack: Some(stack.clone()),
            ..Views::default()
        };
        let apps_page: gtk::Widget = match layout {
            Layout::Compact => self.compact_page(&mut views).upcast(),
            Layout::Classic => self.classic_page(&mut views).upcast(),
            Layout::Grid => self.grid_page(&mut views).upcast(),
        };
        stack.add_named(&apps_page, Some("apps"));
        stack.add_named(&self.results_page(&mut views), Some("results"));

        let header = self.header();
        let panel = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA)
            .ok()
            .map(|settings| PanelLayout::from_settings(&settings))
            .unwrap_or_default();
        if panel.edge == PanelEdge::Bottom {
            root.append(&header);
            root.append(&stack);
            root.append(&search);
        } else {
            root.append(&search);
            root.append(&stack);
            root.append(&header);
        }
        content.append(&root);
        self.state.views.replace(views);

        let cloned = self.clone();
        search.connect_search_changed(move |_| cloned.refresh());
        let cloned = self.clone();
        search.connect_activate(move |_| cloned.activate_default());
        let keys = gtk::EventControllerKey::new();
        let cloned = self.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if matches!(key, gtk::gdk::Key::Down | gtk::gdk::Key::Up) {
                cloned.focus_first();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        search.add_controller(keys);
        self.refresh();
    }

    fn header(&self) -> gtk::Box {
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        header.add_css_class("asena-menu-header");
        let user = gtk::Button::new();
        user.add_css_class("flat");
        user.add_css_class("asena-menu-user");
        user.set_hexpand(true);
        let identity = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let avatar = adw::Avatar::new(30, Some(&self.state.user.name()), true);
        self.state
            .user
            .bind_property("name", &avatar, "text")
            .sync_create()
            .build();
        let photo = gtk::Image::new();
        photo
            .bind_property("paintable", &avatar, "custom-image")
            .sync_create()
            .build();
        let path = self.state.user.icon_file();
        if !path.is_empty() {
            crate::notifications::load_local_image(&photo, &path);
        }
        identity.append(&avatar);
        let name = gtk::Label::new(None);
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        self.state
            .user
            .bind_property("name", &name, "label")
            .sync_create()
            .build();
        identity.append(&name);
        // Keep the photo widget alive with the header.
        identity.append(&photo);
        photo.set_visible(false);
        user.set_child(Some(&identity));
        user.set_tooltip_text(Some(&gettext("Account settings")));
        let window = self.surface.window.downgrade();
        user.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                window.set_visible(false);
            }
            launch_settings(Some("users"));
        });
        header.append(&user);

        let settings = gtk::Button::from_icon_name("preferences-system-symbolic");
        settings.add_css_class("asena-icon-button");
        settings.set_tooltip_text(Some(&gettext("Settings")));
        let window = self.surface.window.downgrade();
        settings.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                window.set_visible(false);
            }
            launch_settings(None);
        });
        header.append(&settings);
        header.append(&crate::power::lock_button(
            &self.surface.window,
            &self.state.daemon,
        ));
        let power = gtk::MenuButton::new();
        power.set_icon_name("system-shutdown-symbolic");
        power.add_css_class("asena-icon-button");
        power.set_tooltip_text(Some(&gettext("Power")));
        crate::power::attach(&power, &self.surface.window, &self.state.daemon);
        header.append(&power);
        header
    }

    fn compact_page(&self, views: &mut Views) -> gtk::Overlay {
        let list = gtk::ListBox::new();
        list.add_css_class("asena-menu-list");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(true);
        let cloned = self.clone();
        list.connect_row_activated(move |_, row| {
            let name = row.widget_name();
            match name.strip_prefix("letter:") {
                Some(_) => cloned.show_picker(),
                None => cloned.launch_id(&name),
            }
        });
        self.attach_context(&list);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .hexpand(true)
            .vexpand(true)
            .build();
        // Letter picker: covers the list while choosing a section.
        let picker_grid = gtk::FlowBox::new();
        picker_grid.set_selection_mode(gtk::SelectionMode::None);
        picker_grid.set_homogeneous(true);
        picker_grid.set_min_children_per_line(5);
        picker_grid.set_max_children_per_line(6);
        picker_grid.set_column_spacing(6);
        picker_grid.set_row_spacing(6);
        picker_grid.set_valign(gtk::Align::Center);
        picker_grid.set_halign(gtk::Align::Center);
        let picker = gtk::Box::new(gtk::Orientation::Vertical, 0);
        picker.add_css_class("asena-menu-picker");
        picker.append(&picker_grid);
        picker.set_visible(false);
        let close = gtk::GestureClick::new();
        let weak = picker.downgrade();
        close.connect_released(move |gesture, _, _, _| {
            if let Some(picker) = weak.upgrade() {
                // Clicks on letters are handled by their buttons.
                if gesture.widget().as_ref() == Some(picker.upcast_ref()) {
                    picker.set_visible(false);
                }
            }
        });
        picker.add_controller(close);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scroll));
        overlay.add_overlay(&picker);
        views.list = Some(list);
        views.list_scroll = Some(scroll);
        views.picker = Some(picker);
        views.picker_grid = Some(picker_grid);
        overlay
    }

    fn show_picker(&self) {
        let (picker, grid, letters) = {
            let views = self.state.views.borrow();
            (
                views.picker.clone(),
                views.picker_grid.clone(),
                views
                    .letters
                    .iter()
                    .map(|(letter, row)| (letter.clone(), row.downgrade()))
                    .collect::<Vec<_>>(),
            )
        };
        let (Some(picker), Some(grid)) = (picker, grid) else {
            return;
        };
        grid.remove_all();
        let mut first = None;
        for letter in launcher::alphabet(launcher::turkish_locale()) {
            let button = gtk::Button::with_label(&letter);
            button.add_css_class("asena-menu-picker-letter");
            let target = letters
                .iter()
                .find(|(name, _)| *name == letter)
                .map(|(_, row)| row.clone());
            button.set_sensitive(target.is_some());
            if target.is_some() && first.is_none() {
                first = Some(button.clone());
            }
            let cloned = self.clone();
            let weak_picker = picker.downgrade();
            button.connect_clicked(move |_| {
                if let Some(picker) = weak_picker.upgrade() {
                    picker.set_visible(false);
                }
                if let Some(row) = target.as_ref().and_then(glib::WeakRef::upgrade) {
                    cloned.scroll_to(&row);
                    row.grab_focus();
                }
            });
            grid.append(&button);
        }
        picker.set_visible(true);
        if let Some(button) = first {
            button.grab_focus();
        }
    }

    fn classic_page(&self, views: &mut Views) -> gtk::Box {
        let page = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let categories = gtk::ListBox::new();
        categories.add_css_class("asena-menu-categories");
        categories.set_selection_mode(gtk::SelectionMode::Single);
        let cloned = self.clone();
        categories.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let key = row.widget_name().to_string();
                if *cloned.state.category.borrow() != key {
                    cloned.state.category.replace(key);
                    cloned.populate_group();
                }
            }
        });
        let side = gtk::ScrolledWindow::builder()
            .child(&categories)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .width_request(176)
            .build();
        page.append(&side);
        let group = gtk::ListBox::new();
        group.add_css_class("asena-menu-list");
        group.set_activate_on_single_click(true);
        let cloned = self.clone();
        group.connect_row_activated(move |_, row| cloned.launch_id(&row.widget_name()));
        self.attach_context(&group);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&group)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .hexpand(true)
            .vexpand(true)
            .build();
        page.append(&scroll);
        views.categories = Some(categories);
        views.group = Some(group);
        page
    }

    fn grid_page(&self, views: &mut Views) -> gtk::ScrolledWindow {
        let grid = gtk::FlowBox::new();
        grid.add_css_class("asena-menu-grid");
        grid.set_selection_mode(gtk::SelectionMode::None);
        grid.set_homogeneous(true);
        grid.set_min_children_per_line(3);
        grid.set_max_children_per_line(6);
        grid.set_valign(gtk::Align::Start);
        grid.set_column_spacing(4);
        grid.set_row_spacing(4);
        views.grid = Some(grid.clone());
        gtk::ScrolledWindow::builder()
            .child(&grid)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build()
    }

    fn results_page(&self, views: &mut Views) -> gtk::ScrolledWindow {
        let results = gtk::ListBox::new();
        results.add_css_class("asena-menu-list");
        results.set_selection_mode(gtk::SelectionMode::Single);
        results.set_activate_on_single_click(true);
        let cloned = self.clone();
        results.connect_row_activated(move |_, row| cloned.activate_result(row.index() as usize));
        let empty = gtk::Box::new(gtk::Orientation::Vertical, 8);
        empty.add_css_class("asena-menu-empty");
        empty.set_valign(gtk::Align::Center);
        empty.set_vexpand(true);
        let icon = gtk::Image::from_icon_name("edit-find-symbolic");
        icon.set_pixel_size(32);
        empty.append(&icon);
        empty.append(&gtk::Label::new(Some(&gettext("No results"))));
        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.append(&results);
        column.append(&empty);
        views.results = Some(results);
        views.empty = Some(empty);
        gtk::ScrolledWindow::builder()
            .child(&column)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build()
    }

    /// Right click (or long press) on an application offers real pin actions.
    fn attach_context(&self, list: &gtk::ListBox) {
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gtk::gdk::BUTTON_SECONDARY);
        let cloned = self.clone();
        let weak = list.downgrade();
        gesture.connect_pressed(move |gesture, _, _, y| {
            let Some(list) = weak.upgrade() else {
                return;
            };
            if let Some(row) = list.row_at_y(y as i32) {
                if row.is_activatable() && !row.widget_name().starts_with("letter:") {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    cloned.context_menu(row.upcast_ref(), &row.widget_name());
                }
            }
        });
        list.add_controller(gesture);
    }

    fn context_menu(&self, anchor: &gtk::Widget, id: &str) {
        let settings = &self.state.settings;
        let in_menu = settings
            .strv("menu-pinned-apps")
            .iter()
            .any(|pin| pin == id);
        let in_panel = settings.strv("pinned-apps").iter().any(|pin| pin == id);
        let model = gio::Menu::new();
        model.append(
            Some(&if in_menu {
                gettext("Unpin from menu")
            } else {
                gettext("Pin to menu")
            }),
            Some("app.menu-pin"),
        );
        model.append(
            Some(&if in_panel {
                gettext("Unpin from taskbar")
            } else {
                gettext("Pin to taskbar")
            }),
            Some("app.panel-pin"),
        );
        let group = gio::SimpleActionGroup::new();
        for (name, key) in [
            ("menu-pin", "menu-pinned-apps"),
            ("panel-pin", "pinned-apps"),
        ] {
            let action = gio::SimpleAction::new(name, None);
            let settings = settings.clone();
            let id = id.to_owned();
            action.connect_activate(move |_, _| {
                let pins: Vec<String> =
                    settings.strv(key).iter().map(ToString::to_string).collect();
                if let Err(error) = settings.set_strv(key, launcher::toggle_pin(&pins, &id)) {
                    tracing::warn!(%error, "Could not save pins");
                }
            });
            group.add_action(&action);
        }
        let popover = gtk::PopoverMenu::from_model(Some(&model));
        popover.set_has_arrow(false);
        popover.insert_action_group("app", Some(&group));
        popover.set_parent(anchor);
        popover.connect_closed(|popover| {
            let popover = popover.clone();
            glib::idle_add_local_once(move || {
                if popover.parent().is_some() {
                    popover.unparent();
                }
            });
        });
        popover.popup();
    }

    fn entries(&self) -> Vec<DesktopEntry> {
        let mut entries: Vec<DesktopEntry> = self.state.apps.with_index(|index| {
            index
                .entries()
                .iter()
                .filter(|entry| entry.visible)
                .cloned()
                .collect()
        });
        collate(&mut entries);
        entries
    }

    fn pins(&self) -> Vec<String> {
        self.state
            .settings
            .strv("menu-pinned-apps")
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn refresh(&self) {
        if let Some(task) = self.state.pending.borrow_mut().take() {
            task.abort();
        }
        let generation = self.state.generation.get().wrapping_add(1);
        self.state.generation.set(generation);
        let (search, stack) = {
            let views = self.state.views.borrow();
            (views.search.clone(), views.stack.clone())
        };
        let (Some(search), Some(stack)) = (search, stack) else {
            return;
        };
        let query = search.text().trim().to_owned();
        if query.is_empty() {
            stack.set_visible_child_name("apps");
            self.populate_apps();
            return;
        }
        stack.set_visible_child_name("results");
        let applications = search::Applications {
            index: self.state.apps.with_index(Clone::clone),
            usage: self.state.usage.borrow().clone(),
        };
        let cloned = self.clone();
        let task = glib::spawn_future_local(async move {
            let query = search::Query { text: query };
            let mut results = applications.query(&query).await;
            results.extend(search::Settings.query(&query).await);
            results.extend(search::Calculator.query(&query).await);
            results.extend(search::Commands.query(&query).await);
            if cloned.state.generation.get() != generation {
                return;
            }
            results.sort_by(|a, b| {
                b.score.cmp(&a.score).then_with(|| {
                    glib::CollationKey::from(a.title.as_str())
                        .cmp(&glib::CollationKey::from(b.title.as_str()))
                })
            });
            cloned.show_results(results);
        });
        self.state.pending.replace(Some(task));
    }

    fn show_results(&self, results: Vec<search::SearchResult>) {
        let (list, empty) = {
            let views = self.state.views.borrow();
            (views.results.clone(), views.empty.clone())
        };
        let (Some(list), Some(empty)) = (list, empty) else {
            return;
        };
        list.remove_all();
        let mut actions = Vec::new();
        empty.set_visible(results.is_empty());
        list.set_visible(!results.is_empty());
        for result in results.into_iter().take(60) {
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let symbolic = result
                .icon
                .as_deref()
                .is_some_and(|icon| icon.ends_with("-symbolic"));
            let icon = icon_for(result.icon.as_deref(), if symbolic { 20 } else { 32 });
            if symbolic {
                icon.set_size_request(32, 32);
            }
            content.append(&icon);
            let title = gtk::Label::new(Some(&result.title));
            title.set_xalign(0.0);
            title.set_hexpand(true);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            content.append(&title);
            let kind = gtk::Label::new(Some(&match &result.action {
                search::Action::Application(_) => gettext("Application"),
                search::Action::Settings(_) => gettext("Settings"),
                search::Action::Copy(_) => gettext("Copy result"),
                search::Action::Command(_) => gettext("Run command"),
            }));
            kind.add_css_class("dim-label");
            kind.add_css_class("caption");
            content.append(&kind);
            let row = gtk::ListBoxRow::new();
            row.add_css_class("asena-menu-app");
            row.set_child(Some(&content));
            list.append(&row);
            actions.push(result.action);
        }
        self.state.actions.replace(actions);
        if let Some(row) = list.row_at_index(0) {
            list.select_row(Some(&row));
        }
    }

    fn populate_apps(&self) {
        match self.state.layout.get() {
            Layout::Compact => self.populate_compact(),
            Layout::Classic => self.populate_classic(),
            Layout::Grid => self.populate_grid(),
        }
    }

    fn populate_compact(&self) {
        let (list, picker) = {
            let views = self.state.views.borrow();
            (views.list.clone(), views.picker.clone())
        };
        let Some(list) = list else {
            return;
        };
        list.remove_all();
        if let Some(picker) = picker {
            picker.set_visible(false);
        }
        let entries = self.entries();
        let pins = self.pins();
        let pinned = launcher::pinned(&entries, &pins);
        if !pinned.is_empty() {
            list.append(&section_row(&gettext("Pinned")));
            for entry in pinned {
                list.append(&app_row(entry, 32));
            }
        }
        list.append(&section_row(&gettext("All applications")));
        let mut letters = Vec::new();
        for (letter, items) in launcher::sections(&entries, launcher::turkish_locale()) {
            let header = section_row(&letter);
            header.add_css_class("asena-menu-letter");
            // A letter header opens the letter picker.
            header.set_activatable(true);
            header.set_focusable(true);
            header.set_widget_name(&format!("letter:{letter}"));
            header.set_tooltip_text(Some(&gettext("Jump to a letter")));
            list.append(&header);
            letters.push((letter.clone(), header));
            for entry in items {
                list.append(&app_row(entry, 32));
            }
        }
        self.state.views.borrow_mut().letters = letters;
    }

    fn scroll_to(&self, row: &gtk::ListBoxRow) {
        let (list, scroll) = {
            let views = self.state.views.borrow();
            (views.list.clone(), views.list_scroll.clone())
        };
        let (Some(list), Some(scroll)) = (list, scroll) else {
            return;
        };
        if let Some(point) = row.compute_point(&list, &gtk::graphene::Point::new(0.0, 0.0)) {
            scroll.vadjustment().set_value(f64::from(point.y()));
        }
    }

    fn populate_classic(&self) {
        let categories = self.state.views.borrow().categories.clone();
        let Some(categories) = categories else {
            return;
        };
        let entries = self.entries();
        let pins = self.pins();
        let selected = self.state.category.borrow().clone();
        categories.remove_all();
        let mut keys = Vec::new();
        if !launcher::pinned(&entries, &pins).is_empty() {
            keys.push(("pinned".to_owned(), gettext("Pinned"), "view-pin-symbolic"));
        }
        keys.push((
            "all".to_owned(),
            gettext("All applications"),
            "view-app-grid-symbolic",
        ));
        for category in Category::ALL {
            if entries
                .iter()
                .any(|entry| launcher::category(&entry.categories) == category)
            {
                keys.push((format!("{category:?}"), category.name(), category.icon()));
            }
        }
        if !keys.iter().any(|(key, _, _)| *key == selected) {
            self.state.category.replace("all".into());
        }
        let selected = self.state.category.borrow().clone();
        for (key, name, icon) in keys {
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            content.append(&gtk::Image::from_icon_name(icon));
            let label = gtk::Label::new(Some(&name));
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            content.append(&label);
            let row = gtk::ListBoxRow::new();
            row.add_css_class("asena-menu-category");
            row.set_child(Some(&content));
            row.set_widget_name(&key);
            categories.append(&row);
            if key == selected {
                categories.select_row(Some(&row));
            }
        }
        self.populate_group();
    }

    fn populate_group(&self) {
        let group = self.state.views.borrow().group.clone();
        let Some(group) = group else {
            return;
        };
        group.remove_all();
        let entries = self.entries();
        let key = self.state.category.borrow().clone();
        let pins = self.pins();
        let items: Vec<&DesktopEntry> = match key.as_str() {
            "pinned" => launcher::pinned(&entries, &pins),
            "all" => entries.iter().collect(),
            key => entries
                .iter()
                .filter(|entry| format!("{:?}", launcher::category(&entry.categories)) == key)
                .collect(),
        };
        for entry in items {
            group.append(&app_row(entry, 32));
        }
    }

    fn populate_grid(&self) {
        let grid = self.state.views.borrow().grid.clone();
        let Some(grid) = grid else {
            return;
        };
        grid.remove_all();
        let entries = self.entries();
        let pins = self.pins();
        let mut ordered: Vec<&DesktopEntry> = launcher::pinned(&entries, &pins);
        ordered.extend(entries.iter().filter(|entry| !pins.contains(&entry.id)));
        for entry in ordered {
            let tile = gtk::Button::new();
            tile.add_css_class("flat");
            tile.add_css_class("asena-menu-tile");
            let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
            content.append(&icon_for(entry.icon.as_deref(), 48));
            let label = gtk::Label::new(Some(&entry.name));
            label.set_wrap(true);
            label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            label.set_lines(2);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_max_width_chars(12);
            label.set_justify(gtk::Justification::Center);
            content.append(&label);
            tile.set_child(Some(&content));
            tile.set_tooltip_text(Some(&entry.name));
            tile.set_widget_name(&entry.id);
            let cloned = self.clone();
            let id = entry.id.clone();
            tile.connect_clicked(move |_| cloned.launch_id(&id));
            let gesture = gtk::GestureClick::new();
            gesture.set_button(gtk::gdk::BUTTON_SECONDARY);
            let cloned = self.clone();
            let weak = tile.downgrade();
            let id = entry.id.clone();
            gesture.connect_pressed(move |gesture, _, _, _| {
                if let Some(tile) = weak.upgrade() {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    cloned.context_menu(tile.upcast_ref(), &id);
                }
            });
            tile.add_controller(gesture);
            grid.append(&tile);
        }
    }

    fn focus_first(&self) {
        let views = self.state.views.borrow();
        let showing_results = views
            .stack
            .as_ref()
            .and_then(gtk::Stack::visible_child_name)
            .as_deref()
            == Some("results");
        let list = if showing_results {
            views.results.clone()
        } else {
            views.list.clone().or_else(|| views.group.clone())
        };
        if let Some(list) = list {
            let mut index = 0;
            while let Some(row) = list.row_at_index(index) {
                if row.is_activatable() && !row.widget_name().starts_with("letter:") {
                    list.select_row(Some(&row));
                    row.grab_focus();
                    return;
                }
                index += 1;
            }
        }
        if let Some(child) = views.grid.as_ref().and_then(|grid| grid.child_at_index(0)) {
            child.grab_focus();
        }
    }

    fn activate_default(&self) {
        let views = self.state.views.borrow();
        if views
            .stack
            .as_ref()
            .and_then(gtk::Stack::visible_child_name)
            .as_deref()
            == Some("results")
        {
            let index = views
                .results
                .as_ref()
                .and_then(gtk::ListBox::selected_row)
                .map(|row| row.index() as usize);
            drop(views);
            if let Some(index) = index {
                self.activate_result(index);
            }
            return;
        }
        let list = views.list.clone().or_else(|| views.group.clone());
        let tile = views
            .grid
            .as_ref()
            .and_then(|grid| grid.child_at_index(0))
            .and_then(|child| child.child());
        drop(views);
        if let Some(list) = list {
            let mut index = 0;
            while let Some(row) = list.row_at_index(index) {
                if row.is_activatable() && !row.widget_name().starts_with("letter:") {
                    self.launch_id(&row.widget_name());
                    return;
                }
                index += 1;
            }
        } else if let Some(tile) = tile {
            self.launch_id(&tile.widget_name());
        }
    }

    fn record(&self, id: &str) {
        let mut usage = self.state.usage.borrow_mut();
        let count = usage.entry(id.to_owned()).or_default();
        *count = count.saturating_add(1);
        let _ = self.state.saves.try_send(usage.clone());
    }

    fn launch_id(&self, id: &str) {
        let entry = self
            .state
            .apps
            .with_index(|index| index.entries().iter().find(|entry| entry.id == id).cloned());
        if let Some(entry) = entry {
            self.record(&entry.id);
            crate::launch::launch(entry, self.surface.content.upcast_ref());
            self.surface.hide();
        }
    }

    fn activate_result(&self, index: usize) {
        let Some(action) = self.state.actions.borrow().get(index).cloned() else {
            return;
        };
        match action {
            search::Action::Application(app) => {
                self.record(&app.id);
                crate::launch::launch(app, self.surface.content.upcast_ref());
            }
            search::Action::Settings(panel) => launch_settings(Some(&panel)),
            search::Action::Copy(text) => self.surface.content.clipboard().set_text(&text),
            search::Action::Command(command) => {
                let parent = self.surface.window.downgrade();
                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(move || {
                        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
                        gio::Subprocess::newv(
                            &[
                                std::ffi::OsStr::new(&shell),
                                std::ffi::OsStr::new("-c"),
                                std::ffi::OsStr::new(&command),
                            ],
                            gio::SubprocessFlags::NONE,
                        )
                        .map(|_| ())
                    })
                    .await;
                    if !matches!(result, Ok(Ok(()))) {
                        tracing::warn!(?result, "Command launch failed");
                        if let Some(parent) = parent.upgrade() {
                            use adw::prelude::*;
                            let dialog = adw::AlertDialog::new(
                                Some(&gettext("Could not run the command")),
                                None,
                            );
                            dialog.add_response("close", &gettext("Close"));
                            dialog.present(Some(&parent));
                        }
                    }
                });
            }
        }
        self.surface.hide();
    }
}
