//! Asena Files window: places sidebar and a monitored folder view.
//!
//! Folder contents come from `gtk::DirectoryList`, which enumerates and
//! watches folders asynchronously through GIO (including `trash:///`).
use crate::places::{name_matches, valid_name, History, SortMode};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const FILE_ATTRIBUTE: &str = "standard::file";
const ATTRIBUTES: &str = "standard::name,standard::display-name,standard::icon,standard::type,\
standard::size,standard::is-hidden,standard::is-backup,standard::content-type,\
time::modified,trash::orig-path,trash::deletion-date";

struct State {
    window: AsenaWindow,
    split: adw::NavigationSplitView,
    title: adw::WindowTitle,
    places: gtk::ListBox,
    directory: gtk::DirectoryList,
    filter: gtk::CustomFilter,
    sorter: gtk::CustomSorter,
    selection: gtk::SingleSelection,
    list: gtk::ListView,
    stack: gtk::Stack,
    empty: adw::StatusPage,
    error: adw::StatusPage,
    search: gtk::SearchEntry,
    trash_bar: gtk::ActionBar,
    history: RefCell<History>,
    show_hidden: Cell<bool>,
    sort: Cell<SortMode>,
    query: RefCell<String>,
}

#[derive(Clone)]
pub struct FilesWindow {
    state: Rc<State>,
}

fn info_file(info: &gio::FileInfo) -> Option<gio::File> {
    info.attribute_object(FILE_ATTRIBUTE)
        .and_then(|object| object.downcast::<gio::File>().ok())
}

fn is_trash(file: &gio::File) -> bool {
    file.uri_scheme().as_deref() == Some("trash")
}

fn detail(info: &gio::FileInfo) -> String {
    let date = info.modification_date_time().and_then(|date| {
        let local = date.to_local().ok()?;
        let today = glib::DateTime::now_local().ok()?;
        let format = if local.ymd() == today.ymd() {
            "%H:%M"
        } else {
            "%e %b %Y"
        };
        local.format(format).ok().map(|text| text.trim().to_owned())
    });
    let kind = if info.file_type() == gio::FileType::Directory {
        gettext("Folder")
    } else {
        glib::format_size(info.size().max(0) as u64).to_string()
    };
    match date {
        Some(date) => format!("{kind} · {date}"),
        None => kind,
    }
}

fn row_widget() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("asena-file-row");
    row.set_margin_top(6);
    row.set_margin_bottom(6);
    row.set_margin_start(12);
    row.set_margin_end(12);
    let image = gtk::Image::new();
    image.set_pixel_size(32);
    row.append(&image);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_valign(gtk::Align::Center);
    text.set_hexpand(true);
    let name = gtk::Label::new(None);
    name.set_xalign(0.0);
    name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    text.append(&name);
    let detail = gtk::Label::new(None);
    detail.set_xalign(0.0);
    detail.add_css_class("dim-label");
    detail.add_css_class("caption");
    detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&detail);
    row.append(&text);
    row
}

fn place_row(icon: &impl IsA<gio::Icon>, label: &str, uri: &str) -> gtk::ListBoxRow {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&gtk::Image::from_gicon(icon));
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
    content.append(&text);
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&content));
    row.set_widget_name(uri);
    row.set_tooltip_text(Some(label));
    row
}

impl FilesWindow {
    pub fn new(app: &adw::Application) -> Self {
        let split = adw::NavigationSplitView::new();
        split.set_min_sidebar_width(200.0);
        split.set_max_sidebar_width(260.0);

        // Places -------------------------------------------------------------
        let places = gtk::ListBox::new();
        places.add_css_class("navigation-sidebar");
        let sidebar_view = adw::ToolbarView::new();
        sidebar_view.add_top_bar(&adw::HeaderBar::new());
        sidebar_view.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .child(&places)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
        ));
        split.set_sidebar(Some(
            &adw::NavigationPage::builder()
                .title(gettext("Places"))
                .tag("places")
                .child(&sidebar_view)
                .build(),
        ));

        // Folder view ----------------------------------------------------------
        let directory = gtk::DirectoryList::new(Some(ATTRIBUTES), None::<&gio::File>);
        directory.set_monitored(true);
        let filter = gtk::CustomFilter::new(|_| true);
        let filtered = gtk::FilterListModel::new(Some(directory.clone()), Some(filter.clone()));
        let sorter = gtk::CustomSorter::new(|_, _| gtk::Ordering::Equal);
        let sorted = gtk::SortListModel::new(Some(filtered), Some(sorter.clone()));
        let selection = gtk::SingleSelection::new(Some(sorted));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            if let Some(item) = item.downcast_ref::<gtk::ListItem>() {
                item.set_child(Some(&row_widget()));
            }
        });
        factory.connect_bind(|_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let (Some(info), Some(row)) = (
                item.item().and_downcast::<gio::FileInfo>(),
                item.child().and_downcast::<gtk::Box>(),
            ) else {
                return;
            };
            row.set_widget_name(&item.position().to_string());
            if let Some(image) = row.first_child().and_downcast::<gtk::Image>() {
                match info.icon() {
                    Some(icon) => image.set_from_gicon(&icon),
                    None => image.set_icon_name(Some("text-x-generic")),
                }
            }
            if let Some(text) = row.last_child() {
                if let Some(name) = text.first_child().and_downcast::<gtk::Label>() {
                    name.set_text(&info.display_name());
                }
                if let Some(label) = text.last_child().and_downcast::<gtk::Label>() {
                    label.set_text(&detail(&info));
                }
            }
        });
        let list = gtk::ListView::new(Some(selection.clone()), Some(factory));
        list.add_css_class("navigation-sidebar");
        list.add_css_class("asena-file-list");
        list.set_single_click_activate(false);

        let empty = adw::StatusPage::builder()
            .icon_name("folder-symbolic")
            .title(gettext("Folder is empty"))
            .build();
        let error = adw::StatusPage::builder()
            .icon_name("dialog-warning-symbolic")
            .title(gettext("Could not open the folder"))
            .build();
        let stack = gtk::Stack::new();
        stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("list"),
        );
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&error, Some("error"));

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search in this folder")));
        search.set_hexpand(true);
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(
            &adw::Clamp::builder()
                .child(&search)
                .maximum_size(480)
                .build(),
        ));
        search_bar.connect_entry(&search);

        let title = adw::WindowTitle::new(&gettext("Files"), "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&title));
        let back = gtk::Button::from_icon_name("go-previous-symbolic");
        back.set_tooltip_text(Some(&gettext("Back")));
        back.set_action_name(Some("win.back"));
        let forward = gtk::Button::from_icon_name("go-next-symbolic");
        forward.set_tooltip_text(Some(&gettext("Forward")));
        forward.set_action_name(Some("win.forward"));
        let up = gtk::Button::from_icon_name("go-up-symbolic");
        up.set_tooltip_text(Some(&gettext("Parent folder")));
        up.set_action_name(Some("win.up"));
        header.pack_start(&back);
        header.pack_start(&forward);
        header.pack_start(&up);
        let menu = gio::Menu::new();
        let view = gio::Menu::new();
        view.append(Some(&gettext("Show hidden files")), Some("win.show-hidden"));
        menu.append_section(None, &view);
        let sort = gio::Menu::new();
        sort.append(Some(&gettext("Sort by name")), Some("win.sort::name"));
        sort.append(
            Some(&gettext("Sort by date modified")),
            Some("win.sort::modified"),
        );
        sort.append(Some(&gettext("Sort by size")), Some("win.sort::size"));
        menu.append_section(None, &sort);
        let other = gio::Menu::new();
        other.append(Some(&gettext("New folder")), Some("win.new-folder"));
        other.append(Some(&gettext("About Files")), Some("app.about"));
        menu.append_section(None, &other);
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let search_toggle = gtk::ToggleButton::new();
        search_toggle.set_icon_name("edit-find-symbolic");
        search_toggle.set_tooltip_text(Some(&gettext("Search")));
        search_toggle
            .bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();
        header.pack_end(&search_toggle);

        let trash_bar = gtk::ActionBar::new();
        let empty_trash = gtk::Button::with_label(&gettext("Empty trash"));
        empty_trash.add_css_class("destructive-action");
        empty_trash.set_action_name(Some("win.empty-trash"));
        trash_bar.pack_end(&empty_trash);
        trash_bar.set_revealed(false);

        let content = adw::ToolbarView::new();
        content.add_top_bar(&header);
        content.add_top_bar(&search_bar);
        content.set_content(Some(&stack));
        content.add_bottom_bar(&trash_bar);
        split.set_content(Some(
            &adw::NavigationPage::builder()
                .title(gettext("Files"))
                .tag("folder")
                .child(&content)
                .build(),
        ));

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Files"))
            .icon_name("org.asenalinux.Files")
            .default_size(960, 620)
            .min_size(360, 360)
            .content(&split)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            640.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            window,
            split,
            title,
            places,
            directory,
            filter,
            sorter,
            selection,
            list,
            stack,
            empty,
            error,
            search,
            trash_bar,
            history: RefCell::default(),
            show_hidden: Cell::new(false),
            sort: Cell::new(SortMode::Name),
            query: RefCell::default(),
        });
        let this = Self { state };
        this.setup_model();
        this.setup_places();
        this.setup_actions();
        this.setup_context_menu();
        this.navigate(&gio::File::for_path(glib::home_dir()), false);
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    /// Opens a folder, or the folder containing a file.
    pub fn open(&self, file: &gio::File) {
        let this = self.clone();
        let target = file.clone();
        file.query_info_async(
            "standard::type",
            gio::FileQueryInfoFlags::NONE,
            glib::Priority::DEFAULT,
            gio::Cancellable::NONE,
            move |result| {
                let folder = match result {
                    Ok(info) if info.file_type() == gio::FileType::Directory => Some(target),
                    _ => target.parent(),
                };
                if let Some(folder) = folder {
                    this.navigate(&folder, true);
                }
            },
        );
    }

    fn current(&self) -> Option<gio::File> {
        self.state.directory.file()
    }

    fn setup_model(&self) {
        let weak = Rc::downgrade(&self.state);
        self.state.filter.set_filter_func(move |item| {
            let (Some(state), Some(info)) = (weak.upgrade(), item.downcast_ref::<gio::FileInfo>())
            else {
                return true;
            };
            (state.show_hidden.get() || !(info.is_hidden() || info.is_backup()))
                && name_matches(&info.display_name(), &state.query.borrow())
        });
        let weak = Rc::downgrade(&self.state);
        self.state.sorter.set_sort_func(move |a, b| {
            let (Some(state), Some(a), Some(b)) = (
                weak.upgrade(),
                a.downcast_ref::<gio::FileInfo>(),
                b.downcast_ref::<gio::FileInfo>(),
            ) else {
                return gtk::Ordering::Equal;
            };
            let folder = |info: &gio::FileInfo| info.file_type() == gio::FileType::Directory;
            let by_name = || {
                glib::CollationKey::from(a.display_name().as_str())
                    .cmp(&glib::CollationKey::from(b.display_name().as_str()))
            };
            let order = match (folder(a), folder(b)) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match state.sort.get() {
                    SortMode::Name => by_name(),
                    SortMode::Modified => b
                        .modification_date_time()
                        .map(|date| date.to_unix())
                        .cmp(&a.modification_date_time().map(|date| date.to_unix()))
                        .then_with(by_name),
                    SortMode::Size => b.size().cmp(&a.size()).then_with(by_name),
                },
            };
            order.into()
        });
        let weak = Rc::downgrade(&self.state);
        self.state.search.connect_search_changed(move |entry| {
            if let Some(state) = weak.upgrade() {
                state.query.replace(entry.text().to_string());
                state.filter.changed(gtk::FilterChange::Different);
            }
        });
        let this = self.clone();
        self.state
            .selection
            .connect_items_changed(move |_, _, _, _| this.update_view());
        let this = self.clone();
        self.state
            .directory
            .connect_loading_notify(move |_| this.update_view());
        let this = self.clone();
        self.state
            .directory
            .connect_error_notify(move |_| this.update_view());
        let this = self.clone();
        self.state.list.connect_activate(move |_, position| {
            let info = this
                .state
                .selection
                .item(position)
                .and_downcast::<gio::FileInfo>();
            if let Some(info) = info {
                this.open_item(&info);
            }
        });
    }

    fn update_view(&self) {
        let state = &self.state;
        if let Some(error) = state.directory.error() {
            state.error.set_description(Some(error.message()));
            state.stack.set_visible_child_name("error");
            return;
        }
        let empty = state.selection.n_items() == 0 && !state.directory.is_loading();
        if empty {
            let searching = !state.query.borrow().trim().is_empty();
            state.empty.set_title(&if searching {
                gettext("No matching files")
            } else if self.current().as_ref().is_some_and(is_trash) {
                gettext("Trash is empty")
            } else {
                gettext("Folder is empty")
            });
        }
        state
            .stack
            .set_visible_child_name(if empty { "empty" } else { "list" });
        if let Some(action) = state
            .window
            .lookup_action("empty-trash")
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(state.selection.n_items() > 0);
        }
    }

    fn navigate(&self, folder: &gio::File, record: bool) {
        let state = &self.state;
        if let Some(current) = self.current() {
            if current.equal(folder) {
                state.split.set_show_content(true);
                return;
            }
            if record {
                state.history.borrow_mut().visit(&current.uri());
            }
        }
        state.search.set_text("");
        state.selection.set_selected(gtk::INVALID_LIST_POSITION);
        state.directory.set_file(Some(folder));
        let trash = is_trash(folder);
        state.trash_bar.set_revealed(trash);
        let home = gio::File::for_path(glib::home_dir());
        let (name, subtitle) = if folder.equal(&home) {
            (gettext("Home"), String::new())
        } else if trash {
            (gettext("Trash"), String::new())
        } else if folder.parent().is_none() && folder.uri_scheme().as_deref() == Some("file") {
            (gettext("Computer"), String::new())
        } else {
            (
                folder
                    .basename()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| folder.parse_name().to_string()),
                folder
                    .parent()
                    .map(|parent| parent.parse_name().to_string())
                    .unwrap_or_default(),
            )
        };
        state.title.set_title(&name);
        state.title.set_subtitle(&subtitle);
        state.window.set_title(Some(&name));
        self.sync_navigation();
        self.select_place(folder);
        state.split.set_show_content(true);
        self.update_view();
    }

    fn sync_navigation(&self) {
        let state = &self.state;
        let history = state.history.borrow();
        let parent = self.current().and_then(|file| file.parent());
        for (name, enabled) in [
            ("back", history.can_back()),
            ("forward", history.can_forward()),
            ("up", parent.is_some()),
        ] {
            if let Some(action) = state
                .window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                action.set_enabled(enabled);
            }
        }
    }

    fn select_place(&self, folder: &gio::File) {
        let uri = folder.uri();
        let places = &self.state.places;
        let mut index = 0;
        places.unselect_all();
        while let Some(row) = places.row_at_index(index) {
            if row.widget_name() == uri {
                places.select_row(Some(&row));
                break;
            }
            index += 1;
        }
    }

    fn setup_places(&self) {
        let places = self.state.places.clone();
        let rebuild = {
            let places = places.clone();
            move || {
                places.remove_all();
                let home = glib::home_dir();
                let named = |name: &str| gio::ThemedIcon::new(name);
                places.append(&place_row(
                    &named("user-home-symbolic"),
                    &gettext("Home"),
                    &gio::File::for_path(&home).uri(),
                ));
                for (directory, icon, label) in [
                    (
                        glib::UserDirectory::Desktop,
                        "user-desktop-symbolic",
                        gettext("Desktop"),
                    ),
                    (
                        glib::UserDirectory::Documents,
                        "folder-documents-symbolic",
                        gettext("Documents"),
                    ),
                    (
                        glib::UserDirectory::Downloads,
                        "folder-download-symbolic",
                        gettext("Downloads"),
                    ),
                    (
                        glib::UserDirectory::Music,
                        "folder-music-symbolic",
                        gettext("Music"),
                    ),
                    (
                        glib::UserDirectory::Pictures,
                        "folder-pictures-symbolic",
                        gettext("Pictures"),
                    ),
                    (
                        glib::UserDirectory::Videos,
                        "folder-videos-symbolic",
                        gettext("Videos"),
                    ),
                ] {
                    if let Some(path) = glib::user_special_dir(directory) {
                        if path != home && path.is_dir() {
                            places.append(&place_row(
                                &named(icon),
                                &label,
                                &gio::File::for_path(path).uri(),
                            ));
                        }
                    }
                }
                places.append(&place_row(
                    &named("user-trash-symbolic"),
                    &gettext("Trash"),
                    "trash:///",
                ));
                places.append(&place_row(
                    &named("drive-harddisk-symbolic"),
                    &gettext("Computer"),
                    &gio::File::for_path("/").uri(),
                ));
                for mount in gio::VolumeMonitor::get().mounts() {
                    if mount.is_shadowed() {
                        continue;
                    }
                    let root = mount.root();
                    if root.uri_scheme().as_deref() != Some("file") {
                        continue;
                    }
                    let row = place_row(&mount.symbolic_icon(), &mount.name(), &root.uri());
                    places.append(&row);
                }
            }
        };
        rebuild();
        let monitor = gio::VolumeMonitor::get();
        let refresh = Rc::new(rebuild);
        let on_change = refresh.clone();
        monitor.connect_mount_added(move |_, _| on_change());
        let on_change = refresh.clone();
        monitor.connect_mount_removed(move |_, _| on_change());
        // The monitor is a singleton; keep it alive with the window.
        let keep = monitor.clone();
        self.state.window.connect_destroy(move |_| {
            let _ = &keep;
        });
        let this = self.clone();
        places.connect_row_activated(move |_, row| {
            let file = gio::File::for_uri(&row.widget_name());
            this.navigate(&file, true);
        });
    }

    fn setup_actions(&self) {
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
            "back",
            &["<Alt>Left", "Back"],
            Box::new(|this| {
                let Some(current) = this.current() else {
                    return;
                };
                let target = this.state.history.borrow_mut().back(&current.uri());
                if let Some(uri) = target {
                    this.navigate(&gio::File::for_uri(&uri), false);
                }
            }),
        );
        add(
            "forward",
            &["<Alt>Right", "Forward"],
            Box::new(|this| {
                let Some(current) = this.current() else {
                    return;
                };
                let target = this.state.history.borrow_mut().forward(&current.uri());
                if let Some(uri) = target {
                    this.navigate(&gio::File::for_uri(&uri), false);
                }
            }),
        );
        add(
            "up",
            &["<Alt>Up"],
            Box::new(|this| {
                if let Some(parent) = this.current().and_then(|file| file.parent()) {
                    this.navigate(&parent, true);
                }
            }),
        );
        add(
            "search",
            &["<Control>f"],
            Box::new(|this| {
                this.state.search.grab_focus();
            }),
        );
        add(
            "new-folder",
            &["<Control><Shift>n"],
            Box::new(|this| this.new_folder()),
        );
        add(
            "rename",
            &["F2"],
            Box::new(|this| {
                if let Some(info) = this.selected() {
                    this.rename(&info);
                }
            }),
        );
        add(
            "trash",
            &["Delete"],
            Box::new(|this| {
                if let Some(info) = this.selected() {
                    this.trash_or_delete(&info);
                }
            }),
        );
        add(
            "open",
            &[],
            Box::new(|this| {
                if let Some(info) = this.selected() {
                    this.open_item(&info);
                }
            }),
        );
        add(
            "open-with",
            &[],
            Box::new(|this| {
                if let Some(file) = this.selected().as_ref().and_then(info_file) {
                    this.launch(&file, true);
                }
            }),
        );
        add(
            "copy-location",
            &[],
            Box::new(|this| {
                if let Some(file) = this.selected().as_ref().and_then(info_file) {
                    this.state.window.clipboard().set_text(&file.parse_name());
                    this.state.window.show_toast(&gettext("Location copied"));
                }
            }),
        );
        add("restore", &[], Box::new(|this| this.restore()));
        add("empty-trash", &[], Box::new(|this| this.empty_trash()));
        add(
            "reload",
            &["F5", "<Control>r"],
            Box::new(|this| {
                if let Some(file) = this.current() {
                    this.state.directory.set_file(None::<&gio::File>);
                    this.state.directory.set_file(Some(&file));
                }
            }),
        );
        let hidden = gio::SimpleAction::new_stateful("show-hidden", None, &false.to_variant());
        let this = self.clone();
        hidden.connect_activate(move |action, _| {
            let value = !this.state.show_hidden.get();
            this.state.show_hidden.set(value);
            action.set_state(&value.to_variant());
            this.state.filter.changed(gtk::FilterChange::Different);
        });
        window.add_action(&hidden);
        if let Some(app) = window.application() {
            app.set_accels_for_action("win.show-hidden", &["<Control>h"]);
        }
        let sort = gio::SimpleAction::new_stateful(
            "sort",
            Some(glib::VariantTy::STRING),
            &"name".to_variant(),
        );
        let this = self.clone();
        sort.connect_activate(move |action, parameter| {
            if let Some(key) = parameter.and_then(|value| value.get::<String>()) {
                let mode = SortMode::from_key(&key);
                this.state.sort.set(mode);
                action.set_state(&mode.key().to_variant());
                this.state.sorter.changed(gtk::SorterChange::Different);
            }
        });
        window.add_action(&sort);
    }

    fn selected(&self) -> Option<gio::FileInfo> {
        self.state
            .selection
            .selected_item()
            .and_downcast::<gio::FileInfo>()
    }

    fn setup_context_menu(&self) {
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gtk::gdk::BUTTON_SECONDARY);
        let this = self.clone();
        gesture.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            let list = &this.state.list;
            let mut current = list.pick(x, y, gtk::PickFlags::DEFAULT);
            let mut position = None;
            while let Some(widget) = current {
                if widget.has_css_class("asena-file-row") {
                    position = widget.widget_name().parse::<u32>().ok();
                    break;
                }
                current = widget.parent();
            }
            let trash = this.current().as_ref().is_some_and(is_trash);
            let menu = gio::Menu::new();
            match position {
                Some(position) => {
                    this.state.selection.set_selected(position);
                    let open = gio::Menu::new();
                    if trash {
                        open.append(Some(&gettext("Restore")), Some("win.restore"));
                        menu.append_section(None, &open);
                        let danger = gio::Menu::new();
                        danger.append(Some(&gettext("Delete permanently")), Some("win.trash"));
                        menu.append_section(None, &danger);
                    } else {
                        open.append(Some(&gettext("Open")), Some("win.open"));
                        open.append(Some(&gettext("Open with…")), Some("win.open-with"));
                        menu.append_section(None, &open);
                        let edit = gio::Menu::new();
                        edit.append(Some(&gettext("Rename…")), Some("win.rename"));
                        edit.append(Some(&gettext("Copy location")), Some("win.copy-location"));
                        edit.append(Some(&gettext("Move to trash")), Some("win.trash"));
                        menu.append_section(None, &edit);
                    }
                }
                None => {
                    this.state
                        .selection
                        .set_selected(gtk::INVALID_LIST_POSITION);
                    if !trash {
                        menu.append(Some(&gettext("New folder")), Some("win.new-folder"));
                    }
                    menu.append(Some(&gettext("Show hidden files")), Some("win.show-hidden"));
                    menu.append(Some(&gettext("Reload")), Some("win.reload"));
                }
            }
            let popover = gtk::PopoverMenu::from_model(Some(&menu));
            popover.set_has_arrow(false);
            popover.set_parent(list);
            popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover.connect_closed(|popover| {
                let popover = popover.clone();
                glib::idle_add_local_once(move || {
                    if popover.parent().is_some() {
                        popover.unparent();
                    }
                });
            });
            popover.popup();
        });
        self.state.list.add_controller(gesture);
    }

    fn open_item(&self, info: &gio::FileInfo) {
        let Some(file) = info_file(info) else {
            return;
        };
        if self.current().as_ref().is_some_and(is_trash) {
            return;
        }
        if info.file_type() == gio::FileType::Directory {
            self.navigate(&file, true);
        } else {
            self.launch(&file, false);
        }
    }

    fn launch(&self, file: &gio::File, ask: bool) {
        let launcher = gtk::FileLauncher::new(Some(file));
        launcher.set_always_ask(ask);
        let window = self.state.window.clone();
        launcher.launch(
            Some(self.state.window.window()),
            gio::Cancellable::NONE,
            move |result| {
                if let Err(error) = result {
                    if !error.matches(gtk::DialogError::Dismissed) {
                        show_error_dialog(
                            window.window(),
                            &gettext("Could not open the file"),
                            error.message(),
                        );
                    }
                }
            },
        );
    }

    fn ask_name(
        &self,
        heading: &str,
        initial: &str,
        confirm: &str,
        done: impl Fn(String) + 'static,
    ) {
        let dialog = adw::AlertDialog::new(Some(heading), None);
        let entry = gtk::Entry::new();
        entry.set_text(initial);
        entry.set_activates_default(true);
        dialog.set_extra_child(Some(&entry));
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("confirm", confirm);
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("confirm"));
        dialog.set_close_response("cancel");
        let weak = dialog.downgrade();
        entry.connect_changed(move |entry| {
            if let Some(dialog) = weak.upgrade() {
                dialog.set_response_enabled("confirm", valid_name(&entry.text()));
            }
        });
        dialog.connect_response(Some("confirm"), move |_, _| {
            let name = entry.text().trim().to_owned();
            if valid_name(&name) {
                done(name);
            }
        });
        dialog.present(Some(self.state.window.window()));
    }

    fn new_folder(&self) {
        let Some(folder) = self.current() else {
            return;
        };
        if is_trash(&folder) {
            return;
        }
        let window = self.state.window.clone();
        self.ask_name(
            &gettext("New folder"),
            &gettext("New folder"),
            &gettext("Create"),
            move |name| {
                let window = window.clone();
                folder.child(&name).make_directory_async(
                    glib::Priority::DEFAULT,
                    gio::Cancellable::NONE,
                    move |result| {
                        if let Err(error) = result {
                            show_error_dialog(
                                window.window(),
                                &gettext("Could not create the folder"),
                                error.message(),
                            );
                        }
                    },
                );
            },
        );
    }

    fn rename(&self, info: &gio::FileInfo) {
        let Some(file) = info_file(info) else {
            return;
        };
        if self.current().as_ref().is_some_and(is_trash) {
            return;
        }
        let window = self.state.window.clone();
        self.ask_name(
            &gettext("Rename"),
            &info.display_name(),
            &gettext("Rename"),
            move |name| {
                let window = window.clone();
                file.set_display_name_async(
                    &name,
                    glib::Priority::DEFAULT,
                    gio::Cancellable::NONE,
                    move |result| {
                        if let Err(error) = result {
                            show_error_dialog(
                                window.window(),
                                &gettext("Could not rename the item"),
                                error.message(),
                            );
                        }
                    },
                );
            },
        );
    }

    fn trash_or_delete(&self, info: &gio::FileInfo) {
        let Some(file) = info_file(info) else {
            return;
        };
        let window = self.state.window.clone();
        if self.current().as_ref().is_some_and(is_trash) {
            confirm_dialog(
                self.state.window.window(),
                &gettext("Delete permanently?"),
                &gettext("This item cannot be restored."),
                &gettext("Delete"),
                true,
                move || {
                    let window = window.clone();
                    file.delete_async(
                        glib::Priority::DEFAULT,
                        gio::Cancellable::NONE,
                        move |result| {
                            if let Err(error) = result {
                                show_error_dialog(
                                    window.window(),
                                    &gettext("Could not delete the item"),
                                    error.message(),
                                );
                            }
                        },
                    );
                },
            );
            return;
        }
        let name = info.display_name().to_string();
        file.trash_async(
            glib::Priority::DEFAULT,
            gio::Cancellable::NONE,
            move |result| match result {
                Ok(()) => window
                    .show_toast(&gettext("“{name}” moved to the trash").replace("{name}", &name)),
                Err(error) => show_error_dialog(
                    window.window(),
                    &gettext("Could not move the item to the trash"),
                    error.message(),
                ),
            },
        );
    }

    fn restore(&self) {
        let Some(info) = self.selected() else {
            return;
        };
        let (Some(file), Some(original)) = (
            info_file(&info),
            info.attribute_byte_string("trash::orig-path"),
        ) else {
            return;
        };
        let destination = gio::File::for_path(original.as_str());
        let window = self.state.window.clone();
        glib::spawn_future_local(async move {
            // GIO's synchronous move runs on a worker; GFile is thread safe.
            let result = gio::spawn_blocking(move || {
                file.move_(
                    &destination,
                    gio::FileCopyFlags::NONE,
                    gio::Cancellable::NONE,
                    None,
                )
            })
            .await;
            if let Ok(Err(error)) = result {
                show_error_dialog(
                    window.window(),
                    &gettext("Could not restore the item"),
                    error.message(),
                );
            }
        });
    }

    fn empty_trash(&self) {
        let items: Vec<gio::File> = (0..self.state.directory.n_items())
            .filter_map(|index| {
                self.state
                    .directory
                    .item(index)
                    .and_downcast::<gio::FileInfo>()
            })
            .filter_map(|info| info_file(&info))
            .collect();
        if items.is_empty() {
            return;
        }
        let window = self.state.window.clone();
        confirm_dialog(
            self.state.window.window(),
            &gettext("Empty the trash?"),
            &gettext("All items in the trash will be deleted permanently."),
            &gettext("Empty trash"),
            true,
            move || {
                for file in items {
                    let window = window.clone();
                    file.delete_async(
                        glib::Priority::DEFAULT,
                        gio::Cancellable::NONE,
                        move |result| {
                            if let Err(error) = result {
                                show_error_dialog(
                                    window.window(),
                                    &gettext("Could not delete the item"),
                                    error.message(),
                                );
                            }
                        },
                    );
                }
            },
        );
    }
}
