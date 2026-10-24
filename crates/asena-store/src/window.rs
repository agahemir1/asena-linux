//! Software window: explore by category, installed applications, updates,
//! search, and an application page that installs, removes and opens.
use crate::{
    catalog::{self, App, Icon, Installed, Source},
    discovery, flatpak, packages, screenshots,
};
use asena_app::prelude::*;
use asena_core::{
    launcher::Category,
    packages::{Action, Plan},
};
use futures_lite::StreamExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct State {
    window: AsenaWindow,
    navigation: adw::NavigationView,
    status: gtk::Stack,
    content: gtk::Stack,
    pages: adw::ViewStack,
    search: gtk::SearchEntry,
    results: gtk::ListBox,
    explore: gtk::Box,
    installed_list: gtk::ListBox,
    updates: gtk::Box,
    banner: adw::Banner,
    apps: RefCell<Rc<Vec<App>>>,
    installed: RefCell<Installed>,
    flatpak_refs: RefCell<Vec<flatpak::Ref>>,
    busy: Cell<bool>,
    /// The Flatpak installation or removal of the application page; Cancel stops it.
    task: RefCell<Option<gio::Cancellable>>,
    /// The Flatpak updates of the updates page.
    update_task: RefCell<Option<gio::Cancellable>>,
    /// Refreshes the open application page after a change.
    page_refresh: RefCell<Option<Box<dyn Fn()>>>,
    /// Lists still being filled, by list.
    fills: RefCell<std::collections::HashMap<usize, glib::SourceId>>,
}

#[derive(Clone)]
pub struct StoreWindow {
    state: Rc<State>,
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder().maximum_size(760).child(child).build();
    gtk::ScrolledWindow::builder()
        .child(&clamp)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build()
}

fn boxed_list() -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    list.set_valign(gtk::Align::Start);
    list
}

fn column() -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 18);
    column.set_margin_top(24);
    column.set_margin_bottom(24);
    column.set_margin_start(12);
    column.set_margin_end(12);
    column
}

pub(crate) fn image(icon: Option<&Icon>, size: i32) -> gtk::Image {
    let image = match icon {
        // A file icon goes through the icon theme, which loads it lazily.
        Some(Icon::File(path)) => {
            gtk::Image::from_gicon(&gio::FileIcon::new(&gio::File::for_path(path)))
        }
        Some(Icon::Named(name)) => gtk::Image::from_icon_name(name),
        None => gtk::Image::from_icon_name("application-x-executable"),
    };
    image.set_pixel_size(size);
    image
}

fn dim(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("dim-label");
    label
}

fn permission_text(permission: flatpak::Permission) -> String {
    match permission {
        flatpak::Permission::Network => gettext("Uses the network"),
        flatpak::Permission::AllFiles => gettext("Can read and change all your files"),
        flatpak::Permission::HomeFiles => gettext("Can read and change files in your home folder"),
        flatpak::Permission::Devices => gettext("Can use all devices, including cameras"),
        flatpak::Permission::Audio => gettext("Can play sound and use microphones"),
        flatpak::Permission::LegacyDisplay => {
            gettext("Uses the X11 window system and can see other windows")
        }
        flatpak::Permission::SessionBus => gettext("Talks to other applications in your session"),
        flatpak::Permission::SystemBus => gettext("Talks to system services"),
    }
}

fn plan_text(plan: &Plan) -> String {
    let installs = plan
        .changes
        .iter()
        .filter(|change| change.action != Action::Remove)
        .count();
    let removals: Vec<&str> = plan.removes().map(|change| change.name.as_str()).collect();
    let mut lines = Vec::new();
    if installs > 0 {
        lines.push(
            ngettext(
                "{count} package to install or update",
                "{count} packages to install or update",
                u32::try_from(installs).unwrap_or(u32::MAX),
            )
            .replace("{count}", &installs.to_string()),
        );
    }
    if plan.download_bytes > 0 {
        lines.push(
            gettext("{size} to download")
                .replace("{size}", &glib::format_size(plan.download_bytes)),
        );
    }
    if !removals.is_empty() {
        lines.push(gettext("To remove: {packages}").replace("{packages}", &removals.join(", ")));
    }
    lines.join("\n")
}

/// The desktop file an application opens with, when it is installed.
fn launcher(app: &App) -> Option<gio::DesktopAppInfo> {
    let id = if app.id.ends_with(".desktop") {
        app.id.clone()
    } else {
        format!("{}.desktop", app.id)
    };
    gio::DesktopAppInfo::new(&id)
}

impl StoreWindow {
    pub fn new(app: &adw::Application) -> Self {
        discovery::install_style();
        let pages = adw::ViewStack::new();
        let explore = column();
        pages
            .add_titled(
                &discovery::view(&explore),
                Some("explore"),
                &gettext("Explore"),
            )
            .set_icon_name(Some("system-search-symbolic"));
        let installed_list = boxed_list();
        let installed = column();
        installed.append(&installed_list);
        pages
            .add_titled(
                &scrolled(&installed),
                Some("installed"),
                &gettext("Installed"),
            )
            .set_icon_name(Some("emblem-default-symbolic"));
        let updates = column();
        pages
            .add_titled(&scrolled(&updates), Some("updates"), &gettext("Updates"))
            .set_icon_name(Some("software-update-available-symbolic"));

        let results = boxed_list();
        let results_column = column();
        results_column.append(&results);
        let content = gtk::Stack::new();
        content.add_named(&pages, Some("pages"));
        content.add_named(&scrolled(&results_column), Some("results"));

        let status = gtk::Stack::new();
        status.add_named(
            &LoadingState::new(&gettext("Loading the catalog")),
            Some("loading"),
        );
        status.add_named(&content, Some("ready"));

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search applications")));
        search.set_hexpand(true);
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(
            &adw::Clamp::builder()
                .maximum_size(520)
                .child(&search)
                .build(),
        ));
        search_bar.connect_entry(&search);
        let search_toggle = gtk::ToggleButton::new();
        search_toggle.set_icon_name("system-search-symbolic");
        search_toggle.set_tooltip_text(Some(&gettext("Search")));
        search_toggle
            .bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .build();

        let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh.set_tooltip_text(Some(&gettext("Check for new software and updates")));
        refresh.set_action_name(Some("win.refresh"));
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("Enable Flathub")), Some("win.enable-flathub"));
        menu.append(Some(&gettext("About Software")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));

        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(&pages));
        switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&switcher));
        header.pack_start(&refresh);
        header.pack_end(&menu_button);
        header.pack_end(&search_toggle);
        let switcher_bar = adw::ViewSwitcherBar::new();
        switcher_bar.set_stack(Some(&pages));
        let banner = adw::Banner::new(&gettext("An interrupted software change must be finished."));
        banner.set_button_label(Some(&gettext("Repair")));
        banner.set_action_name(Some("win.repair"));

        let root = adw::ToolbarView::new();
        root.add_top_bar(&header);
        root.add_top_bar(&search_bar);
        root.add_top_bar(&banner);
        root.add_bottom_bar(&switcher_bar);
        root.set_content(Some(&status));
        let navigation = adw::NavigationView::new();
        navigation.add(
            &adw::NavigationPage::builder()
                .title(gettext("Software"))
                .tag("root")
                .child(&root)
                .build(),
        );

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Software"))
            .icon_name("org.asenalinux.Store")
            .default_size(960, 680)
            .min_size(360, 400)
            .content(&navigation)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            620.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&switcher_bar, "reveal", Some(&true.to_value()));
        // Narrow windows move the page switcher to the bottom bar.
        let narrow_title = adw::WindowTitle::new(&gettext("Software"), "");
        breakpoint.add_setter(&header, "title-widget", Some(&narrow_title.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            window,
            navigation,
            status,
            content,
            pages,
            search,
            results,
            explore,
            installed_list,
            updates,
            banner,
            apps: RefCell::default(),
            installed: RefCell::default(),
            flatpak_refs: RefCell::default(),
            busy: Cell::new(false),
            task: RefCell::default(),
            update_task: RefCell::default(),
            page_refresh: RefCell::default(),
            fills: RefCell::default(),
        });
        let this = Self { state };
        this.setup_actions();
        this.setup_search();
        this.reload();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    pub fn show_updates(&self) {
        self.state.navigation.pop_to_tag("root");
        self.state.pages.set_visible_child_name("updates");
    }

    fn setup_actions(&self) {
        let window = self.state.window.window();
        let refresh = gio::SimpleAction::new("refresh", None);
        let this = self.clone();
        refresh.connect_activate(move |_, _| this.refresh_sources());
        window.add_action(&refresh);

        let flathub = gio::SimpleAction::new("enable-flathub", None);
        let this = self.clone();
        flathub.connect_activate(move |_, _| this.enable_flathub());
        window.add_action(&flathub);
        let weak = flathub.downgrade();
        glib::spawn_future_local(async move {
            let present = gio::spawn_blocking(flatpak::has_flathub)
                .await
                .unwrap_or(false);
            if let Some(action) = weak.upgrade() {
                action.set_enabled(!present);
            }
        });

        let repair = gio::SimpleAction::new("repair", None);
        let this = self.clone();
        repair.connect_activate(move |_, _| this.repair());
        window.add_action(&repair);
    }

    fn setup_search(&self) {
        let this = self.clone();
        self.state.search.connect_search_changed(move |entry| {
            let query = entry.text();
            let state = &this.state;
            if query.trim().is_empty() {
                state.content.set_visible_child_name("pages");
                return;
            }
            let apps = state.apps.borrow().clone();
            let matches = catalog::search(&apps, &query);
            this.fill(
                &state.results,
                matches.into_iter().take(100).cloned().collect(),
            );
            state.content.set_visible_child_name("results");
        });
    }

    /// Loads the catalog and the install state, then fills every page.
    fn reload(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let apps = catalog::load().await;
            *this.state.apps.borrow_mut() = Rc::new(apps);
            this.reload_installed().await;
            this.state.status.set_visible_child_name("ready");
            this.reload_updates().await;
            this.watch_repair().await;
        });
    }

    async fn reload_installed(&self) {
        let packages = gio::spawn_blocking(|| {
            std::fs::read_to_string("/var/lib/dpkg/status")
                .map(|status| asena_core::packages::installed_packages(&status))
                .unwrap_or_default()
        })
        .await
        .unwrap_or_default();
        let refs = gio::spawn_blocking(flatpak::installed)
            .await
            .unwrap_or_default();
        *self.state.installed.borrow_mut() = Installed {
            packages,
            flatpaks: refs.iter().map(|r| r.reference.clone()).collect(),
        };
        *self.state.flatpak_refs.borrow_mut() = refs;
        self.fill_explore();
        self.fill_installed();
        if let Some(refresh) = self.state.page_refresh.borrow().as_ref() {
            refresh();
        }
        let query = self.state.search.text();
        if !query.trim().is_empty() {
            let apps = self.state.apps.borrow().clone();
            self.fill(
                &self.state.results,
                catalog::search(&apps, &query)
                    .into_iter()
                    .take(100)
                    .cloned()
                    .collect(),
            );
        }
    }

    fn row(&self, app: &App) -> adw::ActionRow {
        let row = adw::ActionRow::new();
        row.set_use_markup(false);
        row.set_title(&app.name);
        row.set_subtitle(&app.summary);
        row.set_subtitle_lines(2);
        row.add_prefix(&image(app.icon.as_ref(), 40));
        let installed = self.state.installed.borrow().any(app);
        let mut labels: Vec<String> = app.sources.iter().map(Source::label).collect();
        labels.dedup();
        let suffix = if installed {
            gettext("Installed")
        } else {
            labels.join(" · ")
        };
        row.add_suffix(&dim(&suffix));
        row.set_activatable(true);
        let this = self.clone();
        let app = app.clone();
        row.connect_activated(move |_| this.open(&app));
        row
    }

    /// Fills `list` a few rows at a time, so long lists keep the window responsive.
    fn fill(&self, list: &gtk::ListBox, apps: Vec<App>) {
        let key = list.as_ptr() as usize;
        if let Some(previous) = self.state.fills.borrow_mut().remove(&key) {
            previous.remove();
        }
        list.remove_all();
        list.set_visible(!apps.is_empty());
        let mut pending = apps.into_iter();
        let this = self.clone();
        let target = list.clone();
        let source = glib::idle_add_local(move || {
            for app in pending.by_ref().take(40) {
                target.append(&this.row(&app));
            }
            if pending.len() == 0 {
                this.state.fills.borrow_mut().remove(&key);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        self.state.fills.borrow_mut().insert(key, source);
    }

    fn fill_explore(&self) {
        let apps = self.state.apps.borrow().clone();
        let weak = Rc::downgrade(&self.state);
        let category_target = weak.clone();
        discovery::fill(
            &self.state.explore,
            &apps,
            &self.state.installed.borrow(),
            Rc::new(move |app| {
                if let Some(state) = weak.upgrade() {
                    Self { state }.open(app);
                }
            }),
            Rc::new(move |category| {
                if let Some(state) = category_target.upgrade() {
                    Self { state }.open_category(category);
                }
            }),
        );
    }

    fn fill_installed(&self) {
        let apps = self.state.apps.borrow().clone();
        let installed = self.state.installed.borrow().clone();
        self.fill(
            &self.state.installed_list,
            catalog::sorted(apps.iter().filter(|app| installed.any(app)))
                .into_iter()
                .cloned()
                .collect(),
        );
    }

    fn open_category(&self, category: Category) {
        let list = boxed_list();
        let apps = self.state.apps.borrow().clone();
        self.fill(
            &list,
            catalog::sorted(apps.iter().filter(|app| app.category() == category))
                .into_iter()
                .cloned()
                .collect(),
        );
        let content = column();
        content.append(&list);
        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&scrolled(&content)));
        self.state.navigation.push(
            &adw::NavigationPage::builder()
                .title(category.name())
                .child(&view)
                .build(),
        );
    }

    /// The application page.
    fn open(&self, app: &App) {
        let content = column();
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        header.append(&image(app.icon.as_ref(), 96));
        let texts = gtk::Box::new(gtk::Orientation::Vertical, 6);
        texts.set_valign(gtk::Align::Center);
        let name = gtk::Label::new(Some(&app.name));
        name.add_css_class("title-1");
        name.set_wrap(true);
        name.set_xalign(0.0);
        let summary = dim(&app.summary);
        summary.set_wrap(true);
        summary.set_xalign(0.0);
        texts.append(&name);
        texts.append(&summary);
        header.append(&texts);
        content.append(&header);

        let labels: Vec<String> = app.sources.iter().map(Source::label).collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let source = gtk::DropDown::from_strings(&label_refs);
        source.set_visible(app.sources.len() > 1);
        source.set_tooltip_text(Some(&gettext("Source")));
        let primary = gtk::Button::new();
        primary.add_css_class("pill");
        let open = gtk::Button::with_label(&gettext("Open"));
        open.add_css_class("pill");
        // Flatpak transactions can stop midway; APT changes cannot.
        let cancel = gtk::Button::with_label(&gettext("Cancel"));
        cancel.add_css_class("pill");
        cancel.set_visible(false);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        actions.append(&primary);
        actions.append(&open);
        actions.append(&cancel);
        actions.append(&source);
        let progress = gtk::ProgressBar::new();
        progress.set_visible(false);
        content.append(&actions);
        content.append(&progress);

        let carousel = adw::Carousel::new();
        carousel.set_height_request(320);
        carousel.set_spacing(12);
        let dots = adw::CarouselIndicatorDots::new();
        dots.set_carousel(Some(&carousel));
        let gallery = gtk::Box::new(gtk::Orientation::Vertical, 6);
        gallery.append(&carousel);
        gallery.append(&dots);
        gallery.set_visible(false);
        content.append(&gallery);

        if !app.description.is_empty() {
            let description = gtk::Label::new(Some(&app.description));
            description.set_wrap(true);
            description.set_xalign(0.0);
            description.set_selectable(true);
            content.append(&description);
        }
        let details = adw::PreferencesGroup::new();
        if let Some(license) = &app.license {
            let row = adw::ActionRow::new();
            row.set_use_markup(false);
            row.set_title(&gettext("License"));
            row.set_subtitle(license);
            details.add(&row);
        }
        if let Some(homepage) = app
            .homepage
            .as_ref()
            .filter(|url| url.starts_with("https://"))
        {
            let row = adw::ActionRow::new();
            row.set_use_markup(false);
            row.set_title(&gettext("Website"));
            row.set_subtitle(homepage);
            let link = gtk::Button::from_icon_name("adw-external-link-symbolic");
            link.add_css_class("flat");
            link.set_valign(gtk::Align::Center);
            link.set_tooltip_text(Some(&gettext("Open the website")));
            let uri = homepage.clone();
            let window = self.state.window.window().clone();
            link.connect_clicked(move |_| {
                gtk::UriLauncher::new(&uri).launch(Some(&window), gio::Cancellable::NONE, |_| {});
            });
            row.add_suffix(&link);
            details.add(&row);
        }
        content.append(&details);

        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&scrolled(&content)));
        let page = adw::NavigationPage::builder()
            .title(app.name.clone())
            .child(&view)
            .build();

        // Buttons follow the selected source and the install state.
        let update = {
            let this = self.clone();
            let app = app.clone();
            let (source, primary, open) = (source.clone(), primary.clone(), open.clone());
            move || {
                let selected = app
                    .sources
                    .get(source.selected() as usize)
                    .or_else(|| app.sources.first());
                let installed = selected.is_some_and(|s| this.state.installed.borrow().has(s));
                primary.set_label(&if installed {
                    gettext("Remove")
                } else {
                    gettext("Install")
                });
                primary.remove_css_class("suggested-action");
                primary.remove_css_class("destructive-action");
                primary.add_css_class(if installed {
                    "destructive-action"
                } else {
                    "suggested-action"
                });
                primary.set_sensitive(!this.state.busy.get());
                open.set_visible(installed && launcher(&app).is_some());
            }
        };
        update();
        let refresh = update.clone();
        source.connect_selected_notify(move |_| refresh());
        *self.state.page_refresh.borrow_mut() = Some(Box::new(update));
        let this = self.clone();
        page.connect_hidden(move |_| {
            this.state.page_refresh.borrow_mut().take();
        });

        let this = self.clone();
        let chosen = app.clone();
        let progress_bar = progress.clone();
        let source_list = source.clone();
        let cancel_button = cancel.clone();
        primary.connect_clicked(move |_| {
            let Some(selected) = chosen
                .sources
                .get(source_list.selected() as usize)
                .or_else(|| chosen.sources.first())
                .cloned()
            else {
                return;
            };
            let installed = this.state.installed.borrow().has(&selected);
            this.change(&chosen, selected, !installed, &progress_bar, &cancel_button);
        });
        let this = self.clone();
        cancel.connect_clicked(move |_| {
            if let Some(task) = this.state.task.borrow().as_ref() {
                task.cancel();
            }
        });
        let launch_app = app.clone();
        open.connect_clicked(move |button| {
            if let Some(info) = launcher(&launch_app) {
                let context = button.display().app_launch_context();
                if let Err(error) = info.launch(&[], Some(&context)) {
                    tracing::warn!(%error, "Could not open the application");
                }
            }
        });

        for url in app.screenshots.clone() {
            let carousel = carousel.downgrade();
            let gallery = gallery.downgrade();
            glib::spawn_future_local(async move {
                let Some(texture) = screenshots::load(url).await else {
                    return;
                };
                let (Some(carousel), Some(gallery)) = (carousel.upgrade(), gallery.upgrade())
                else {
                    return;
                };
                let picture = gtk::Picture::for_paintable(&texture);
                picture.set_content_fit(gtk::ContentFit::Contain);
                picture.set_hexpand(true);
                carousel.append(&picture);
                gallery.set_visible(true);
            });
        }
        self.state.navigation.push(&page);
    }

    /// Installs a `.flatpakref` or `.deb` file opened from the file manager.
    pub fn open_file(&self, path: std::path::PathBuf) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            match extension.as_str() {
                "flatpakref" => this.install_flatpakref(path).await,
                "deb" => this.install_deb(path).await,
                _ => this.error(
                    &gettext("The file cannot be installed"),
                    &gettext("It is not a software package."),
                ),
            }
        });
    }

    async fn install_flatpakref(&self, path: std::path::PathBuf) {
        let contents = gio::spawn_blocking(move || std::fs::read(path).ok())
            .await
            .ok()
            .flatten()
            .filter(|contents| contents.len() < 1024 * 1024);
        let Some((contents, (title, url))) = contents
            .and_then(|contents| flatpak::describe_ref_file(&contents).map(|d| (contents, d)))
        else {
            self.error(
                &gettext("The file cannot be installed"),
                &gettext("It is not a valid Flatpak reference."),
            );
            return;
        };
        let mut body = gettext("Install applications only from sources you trust.");
        if !url.is_empty() {
            body = format!(
                "{}\n{}",
                gettext("Source: {url}").replace("{url}", &url),
                body
            );
        }
        let heading = gettext("Install {name}?").replace("{name}", &title);
        if self.state.busy.get()
            || !self
                .confirm(&heading, &body, &gettext("Install"), false)
                .await
        {
            return;
        }
        self.set_busy(true);
        self.state
            .window
            .show_toast(&gettext("Installing {name}").replace("{name}", &title));
        let (sender, _receiver) = async_channel::unbounded::<i32>();
        let cancellable = gio::Cancellable::new();
        let bytes = glib::Bytes::from_owned(contents);
        let result =
            gio::spawn_blocking(move || flatpak::install_ref_file(&bytes, &sender, &cancellable))
                .await;
        self.set_busy(false);
        match result {
            Ok(Ok(())) => self
                .state
                .window
                .show_toast(&gettext("{name} is installed").replace("{name}", &title)),
            Ok(Err(error)) => self.error(&gettext("The change failed"), error.message()),
            Err(_) => {}
        }
        self.reload_installed().await;
    }

    async fn install_deb(&self, path: std::path::PathBuf) {
        let lookup = path.clone();
        let Some((name, version, summary)) =
            gio::spawn_blocking(move || packages::describe_deb(&lookup))
                .await
                .ok()
                .flatten()
        else {
            self.error(
                &gettext("The file cannot be installed"),
                &gettext("It is not a valid Debian package."),
            );
            return;
        };
        let warning = format!(
            "{name} {version}\n{summary}\n\n{}",
            gettext("This package does not come from the Asena or Debian archives and is not updated with the system. Install it only if you trust its source.")
        );
        let heading = gettext("Install {name}?").replace("{name}", &name);
        if self.state.busy.get()
            || !self
                .confirm(&heading, &warning, &gettext("Continue"), false)
                .await
        {
            return;
        }
        let plan = match packages::plan_file(path).await {
            Ok(plan) => plan,
            Err(error) => {
                self.error(
                    &gettext("The change cannot be prepared"),
                    &packages::message(&error),
                );
                return;
            }
        };
        if !self
            .confirm(&heading, &plan_text(&plan), &gettext("Install"), false)
            .await
        {
            return;
        }
        self.set_busy(true);
        self.state
            .window
            .show_toast(&gettext("Installing {name}").replace("{name}", &name));
        let result = async { packages::proxy().await?.commit(&plan.id).await }.await;
        self.set_busy(false);
        match result {
            Ok(()) => self
                .state
                .window
                .show_toast(&gettext("{name} is installed").replace("{name}", &name)),
            Err(error) => self.error(&gettext("The change failed"), &packages::message(&error)),
        }
        self.reload_installed().await;
    }

    fn set_busy(&self, busy: bool) {
        self.state.busy.set(busy);
        if let Some(refresh) = self.state.page_refresh.borrow().as_ref() {
            refresh();
        }
    }

    fn error(&self, title: &str, message: &str) {
        show_error_dialog(self.state.window.window(), title, message);
    }

    /// Installs (`install`) or removes the application from `source`.
    fn change(
        &self,
        app: &App,
        source: Source,
        install: bool,
        progress: &gtk::ProgressBar,
        cancel: &gtk::Button,
    ) {
        if self.state.busy.get() {
            return;
        }
        let this = self.clone();
        let app = app.clone();
        let progress = progress.clone();
        let cancel = cancel.clone();
        glib::spawn_future_local(async move {
            match source {
                Source::Package { name, .. } => {
                    this.change_package(&app, &name, install, &progress).await
                }
                Source::Flatpak {
                    reference,
                    remote,
                    user,
                } => {
                    this.change_flatpak(&app, reference, remote, user, install, &progress, &cancel)
                        .await
                }
            }
        });
    }

    async fn confirm(&self, heading: &str, body: &str, label: &str, destructive: bool) -> bool {
        let dialog = adw::AlertDialog::new(Some(heading), Some(body));
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("confirm", label);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance(
            "confirm",
            if destructive {
                adw::ResponseAppearance::Destructive
            } else {
                adw::ResponseAppearance::Suggested
            },
        );
        dialog.choose_future(self.state.window.window()).await == "confirm"
    }

    async fn change_package(
        &self,
        app: &App,
        name: &str,
        install: bool,
        progress: &gtk::ProgressBar,
    ) {
        let (installs, removes): (&[&str], &[&str]) = if install {
            (&[name], &[])
        } else {
            (&[], &[name])
        };
        let plan = match packages::plan(installs, removes).await {
            Ok(plan) => plan,
            Err(error) => {
                self.error(
                    &gettext("The change cannot be prepared"),
                    &packages::message(&error),
                );
                return;
            }
        };
        let (heading, label) = if install {
            (gettext("Install {name}?"), gettext("Install"))
        } else {
            (gettext("Remove {name}?"), gettext("Remove"))
        };
        let heading = heading.replace("{name}", &app.name);
        if !self
            .confirm(&heading, &plan_text(&plan), &label, !install)
            .await
        {
            return;
        }
        self.set_busy(true);
        progress.set_fraction(0.0);
        progress.set_visible(true);
        let result = async {
            let proxy = packages::proxy().await?;
            let mut updates = proxy.receive_progress().await?;
            let bar = progress.clone();
            let follow = async move {
                while let Some(signal) = updates.next().await {
                    if let Ok(args) = signal.args() {
                        if *args.percent() >= 0.0 {
                            bar.set_fraction(args.percent() / 100.0);
                        }
                        bar.set_text(Some(args.detail()));
                        bar.set_show_text(!args.detail().is_empty());
                    }
                }
            };
            let commit = proxy.commit(&plan.id);
            futures_lite::future::or(commit, async {
                follow.await;
                Ok(())
            })
            .await
        }
        .await;
        progress.set_visible(false);
        self.set_busy(false);
        match result {
            Ok(()) => self.state.window.show_toast(&if install {
                gettext("{name} is installed").replace("{name}", &app.name)
            } else {
                gettext("{name} is removed").replace("{name}", &app.name)
            }),
            Err(error) => self.error(&gettext("The change failed"), &packages::message(&error)),
        }
        self.reload_installed().await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn change_flatpak(
        &self,
        app: &App,
        reference: String,
        remote: String,
        user: bool,
        install: bool,
        progress: &gtk::ProgressBar,
        cancel: &gtk::Button,
    ) {
        // An installed application is removed from the installation that has it.
        let user = self
            .state
            .flatpak_refs
            .borrow()
            .iter()
            .find(|r| r.reference == reference)
            .map_or(user, |r| r.user);
        if install {
            let (remote_name, reference_name) = (remote.clone(), reference.clone());
            let permissions = gio::spawn_blocking(move || {
                flatpak::permissions(&remote_name, &reference_name, user)
            })
            .await;
            let permissions = match permissions {
                Ok(Ok(permissions)) => permissions,
                Ok(Err(error)) => {
                    self.error(
                        &gettext("The application cannot be installed"),
                        error.message(),
                    );
                    return;
                }
                Err(_) => return,
            };
            let body = if permissions.is_empty() {
                gettext("It asks for no access outside its sandbox.")
            } else {
                permissions
                    .into_iter()
                    .map(permission_text)
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let heading = gettext("Install {name}?").replace("{name}", &app.name);
            if !self
                .confirm(&heading, &body, &gettext("Install"), false)
                .await
            {
                return;
            }
        } else {
            let heading = gettext("Remove {name}?").replace("{name}", &app.name);
            let body = gettext("Its data in your home folder stays.");
            if !self
                .confirm(&heading, &body, &gettext("Remove"), true)
                .await
            {
                return;
            }
        }
        self.set_busy(true);
        progress.set_fraction(0.0);
        progress.set_visible(true);
        let (sender, receiver) = async_channel::unbounded::<i32>();
        let bar = progress.clone();
        let follow = glib::spawn_future_local(async move {
            while let Ok(percent) = receiver.recv().await {
                bar.set_fraction(f64::from(percent.clamp(0, 100)) / 100.0);
            }
        });
        let cancellable = gio::Cancellable::new();
        self.state.task.replace(Some(cancellable.clone()));
        cancel.set_visible(true);
        let task = cancellable.clone();
        let result = gio::spawn_blocking(move || {
            if install {
                flatpak::install(&remote, &reference, user, &sender, &task)
            } else {
                flatpak::uninstall(&reference, user, &sender, &task)
            }
        })
        .await;
        follow.abort();
        self.state.task.replace(None);
        cancel.set_visible(false);
        progress.set_visible(false);
        self.set_busy(false);
        match result {
            Ok(Ok(())) => self.state.window.show_toast(&if install {
                gettext("{name} is installed").replace("{name}", &app.name)
            } else {
                gettext("{name} is removed").replace("{name}", &app.name)
            }),
            Ok(Err(_)) if cancellable.is_cancelled() => {}
            Ok(Err(error)) => self.error(&gettext("The change failed"), error.message()),
            Err(_) => {}
        }
        self.reload_installed().await;
    }

    /// The updates page: system upgrades applied while restarting and
    /// Flatpak updates applied now.
    async fn reload_updates(&self) {
        let page = &self.state.updates;
        while let Some(child) = page.first_child() {
            page.remove(&child);
        }
        let proxy = packages::proxy().await.ok();
        let scheduled = match &proxy {
            Some(proxy) => proxy.upgrade_scheduled().await.unwrap_or(false),
            None => false,
        };
        let system = packages::upgrades().await;
        let apps = gio::spawn_blocking(flatpak::updates)
            .await
            .unwrap_or_default();

        if let Some(result) = packages::last_upgrade()
            .await
            .filter(|result| !result.success)
        {
            let failed = adw::PreferencesGroup::new();
            let row = adw::ActionRow::new();
            row.set_use_markup(false);
            row.set_title(&gettext("The last system update could not be installed"));
            row.set_subtitle(&result.error.unwrap_or_default());
            failed.add(&row);
            page.append(&failed);
        }

        let group = adw::PreferencesGroup::new();
        group.set_title(&gettext("System"));
        match &system {
            Ok(upgrades) if !upgrades.is_empty() => {
                let button = gtk::Button::with_label(&if scheduled {
                    gettext("Restart now")
                } else {
                    gettext("Restart and install")
                });
                button.add_css_class("suggested-action");
                button.set_valign(gtk::Align::Center);
                let this = self.clone();
                button.connect_clicked(move |_| this.install_system_updates(scheduled));
                group.set_header_suffix(Some(&button));
                if scheduled {
                    group.set_description(Some(&gettext(
                        "The updates are installed during the next restart.",
                    )));
                    let cancel = adw::ActionRow::new();
                    cancel.set_title(&gettext("Do not install while restarting"));
                    cancel.set_activatable(true);
                    let this = self.clone();
                    cancel.connect_activated(move |_| this.cancel_system_updates());
                    group.add(&cancel);
                }
                for upgrade in upgrades {
                    let row = adw::ActionRow::new();
                    row.set_use_markup(false);
                    row.set_title(&upgrade.name);
                    row.set_subtitle(&format!("{} → {}", upgrade.from, upgrade.to));
                    if upgrade.security {
                        row.add_suffix(&dim(&gettext("Security")));
                    }
                    group.add(&row);
                }
            }
            Ok(_) => {
                let row = adw::ActionRow::new();
                row.set_title(&gettext("The system is up to date"));
                group.add(&row);
            }
            Err(error) => {
                let row = adw::ActionRow::new();
                row.set_use_markup(false);
                row.set_title(&gettext("Updates cannot be checked"));
                row.set_subtitle(&packages::message(error));
                group.add(&row);
            }
        }
        page.append(&group);

        let group = adw::PreferencesGroup::new();
        group.set_title(&gettext("Applications"));
        if apps.is_empty() {
            let row = adw::ActionRow::new();
            row.set_title(&gettext("The applications are up to date"));
            group.add(&row);
        } else {
            let button = if self.state.update_task.borrow().is_some() {
                gtk::Button::with_label(&gettext("Cancel"))
            } else {
                let button = gtk::Button::with_label(&gettext("Update"));
                button.add_css_class("suggested-action");
                button
            };
            button.set_valign(gtk::Align::Center);
            let this = self.clone();
            let refs = apps.clone();
            button.connect_clicked(move |button| this.update_flatpaks(refs.clone(), button));
            group.set_header_suffix(Some(&button));
            for app in &apps {
                let row = adw::ActionRow::new();
                row.set_use_markup(false);
                row.set_title(&app.name);
                group.add(&row);
            }
        }
        page.append(&group);
    }

    fn install_system_updates(&self, scheduled: bool) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            if !scheduled {
                let result = async { packages::proxy().await?.schedule_upgrade().await }.await;
                if let Err(error) = result {
                    this.error(
                        &gettext("The updates cannot be prepared"),
                        &packages::message(&error),
                    );
                    return;
                }
                this.reload_updates().await;
            }
            let restart = this
                .confirm(
                    &gettext("Restart now?"),
                    &gettext("The updates are installed while the computer restarts. Save your work first."),
                    &gettext("Restart"),
                    false,
                )
                .await;
            if restart {
                if let Err(error) = packages::restart().await {
                    this.error(&gettext("The computer cannot restart"), &error.to_string());
                }
            }
        });
    }

    fn cancel_system_updates(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Err(error) = async { packages::proxy().await?.cancel_upgrade().await }.await {
                this.error(&gettext("The change failed"), &packages::message(&error));
            }
            this.reload_updates().await;
        });
    }

    /// Starts the Flatpak updates; while they run the same button cancels them.
    fn update_flatpaks(&self, refs: Vec<flatpak::Ref>, button: &gtk::Button) {
        if let Some(task) = self.state.update_task.borrow().as_ref() {
            task.cancel();
            return;
        }
        if self.state.busy.get() {
            return;
        }
        let cancellable = gio::Cancellable::new();
        self.state.update_task.replace(Some(cancellable.clone()));
        self.set_busy(true);
        button.set_label(&gettext("Cancel"));
        button.remove_css_class("suggested-action");
        let this = self.clone();
        glib::spawn_future_local(async move {
            let (sender, _receiver) = async_channel::unbounded::<i32>();
            let task = cancellable.clone();
            let result = gio::spawn_blocking(move || flatpak::update(&refs, &sender, &task)).await;
            this.state.update_task.replace(None);
            this.set_busy(false);
            match result {
                Ok(Err(_)) if cancellable.is_cancelled() => {}
                Ok(Err(error)) => {
                    this.error(
                        &gettext("The applications cannot be updated"),
                        error.message(),
                    );
                }
                _ => {}
            }
            this.reload_updates().await;
        });
    }

    /// Downloads new package lists and Flatpak catalogs, then reloads.
    fn refresh_sources(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.state.status.set_visible_child_name("loading");
            if let Err(error) = async { packages::proxy().await?.refresh().await }.await {
                tracing::warn!(%error, "Package lists were not refreshed");
            }
            let _ = gio::spawn_blocking(flatpak::refresh_catalogs).await;
            this.reload();
        });
    }

    fn enable_flathub(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let accepted = this
                .confirm(
                    &gettext("Enable Flathub?"),
                    &gettext("Flathub is a third-party catalog of Flatpak applications. Asena does not review them; each one shows the access it asks for before you install it."),
                    &gettext("Enable"),
                    false,
                )
                .await;
            if !accepted {
                return;
            }
            this.state.status.set_visible_child_name("loading");
            // Without a network the catalog download fails; offer to retry or skip.
            loop {
                match gio::spawn_blocking(flatpak::enable_flathub).await {
                    Ok(Ok(())) => {
                        if let Some(action) = this
                            .state
                            .window
                            .window()
                            .lookup_action("enable-flathub")
                            .and_downcast::<gio::SimpleAction>()
                        {
                            action.set_enabled(false);
                        }
                        break;
                    }
                    Ok(Err(error)) => {
                        let retry = this
                            .confirm(
                                &gettext("Flathub cannot be enabled"),
                                error.message(),
                                &gettext("Try again"),
                                false,
                            )
                            .await;
                        if !retry {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            this.reload();
        });
    }

    async fn watch_repair(&self) {
        let Ok(proxy) = packages::proxy().await else {
            return;
        };
        self.state
            .banner
            .set_revealed(proxy.needs_repair().await.unwrap_or(false));
    }

    fn repair(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = async { packages::proxy().await?.repair().await }.await;
            match result {
                Ok(()) => this.state.banner.set_revealed(false),
                Err(error) => this.error(&gettext("The repair failed"), &packages::message(&error)),
            }
            this.reload_installed().await;
        });
    }
}
