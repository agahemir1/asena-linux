//! Archive window: list an archive's contents and extract it safely.
use crate::archive::{folder_name, is_safe_extraction_path, parse_listing, TOOL};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

const MAX_ROWS: usize = 5000;

struct State {
    window: AsenaWindow,
    title: adw::WindowTitle,
    stack: gtk::Stack,
    list: gtk::ListBox,
    error: adw::StatusPage,
    extract: gtk::Button,
    archive: RefCell<Option<PathBuf>>,
    entries: RefCell<Vec<String>>,
    extracted: RefCell<Option<PathBuf>>,
}

#[derive(Clone)]
pub struct ArchiveWindow {
    state: Rc<State>,
}

async fn run(args: Vec<String>) -> Result<String, String> {
    let argv: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
    let process = gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|error| error.message().to_owned())?;
    let (stdout, stderr) = process
        .communicate_utf8_future(None)
        .await
        .map_err(|error| error.message().to_owned())?;
    if process.is_successful() {
        Ok(stdout.map(|text| text.to_string()).unwrap_or_default())
    } else {
        Err(stderr
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| gettext("The archive could not be read.")))
    }
}

impl ArchiveWindow {
    pub fn new(app: &adw::Application) -> Self {
        let title = adw::WindowTitle::new(&gettext("Archive"), "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&title));
        let open = gtk::Button::with_label(&gettext("Open"));
        open.set_action_name(Some("win.open"));
        header.pack_start(&open);
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("About Archive")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let extract = gtk::Button::with_label(&gettext("Extract"));
        extract.add_css_class("suggested-action");
        extract.set_action_name(Some("win.extract"));
        extract.set_sensitive(false);
        header.pack_end(&extract);

        let start = adw::StatusPage::builder()
            .icon_name("package-x-generic-symbolic")
            .title(gettext("Open an archive"))
            .build();
        let start_button = gtk::Button::with_label(&gettext("Open…"));
        start_button.add_css_class("pill");
        start_button.add_css_class("suggested-action");
        start_button.set_halign(gtk::Align::Center);
        start_button.set_action_name(Some("win.open"));
        start.set_child(Some(&start_button));
        let missing = adw::StatusPage::builder()
            .icon_name("dialog-warning-symbolic")
            .title(gettext("Archive support is not installed"))
            .description(gettext(
                "Install the libarchive-tools package to open archives.",
            ))
            .build();
        let error = adw::StatusPage::builder()
            .icon_name("dialog-error-symbolic")
            .title(gettext("Could not open the archive"))
            .build();
        let loading = adw::Spinner::new();
        loading.set_size_request(48, 48);
        loading.set_halign(gtk::Align::Center);
        loading.set_valign(gtk::Align::Center);
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        list.set_selection_mode(gtk::SelectionMode::None);
        let stack = gtk::Stack::new();
        stack.add_named(&start, Some("start"));
        stack.add_named(&missing, Some("missing"));
        stack.add_named(&error, Some("error"));
        stack.add_named(&loading, Some("loading"));
        stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("list"),
        );
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Archive"))
            .default_size(720, 520)
            .min_size(360, 320)
            .content(&toolbar)
            .build();
        let state = Rc::new(State {
            window,
            title,
            stack,
            list,
            error,
            extract,
            archive: RefCell::default(),
            entries: RefCell::default(),
            extracted: RefCell::default(),
        });
        let this = Self { state };
        this.setup_actions();
        if glib::find_program_in_path(TOOL).is_none() {
            this.state.stack.set_visible_child_name("missing");
        }
        this
    }

    pub fn present(&self) {
        self.state.window.present();
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
            "open",
            &["<Control>o"],
            Box::new(|this| {
                let dialog = gtk::FileDialog::new();
                let parent = this.state.window.window().clone();
                let this = this.clone();
                dialog.open(Some(&parent), gio::Cancellable::NONE, move |result| {
                    if let Ok(file) = result {
                        this.open(&file);
                    }
                });
            }),
        );
        add(
            "extract",
            &["<Control>e"],
            Box::new(|this| this.choose_destination()),
        );
        add(
            "show-extracted",
            &[],
            Box::new(|this| {
                if let Some(path) = this.state.extracted.borrow().clone() {
                    gtk::FileLauncher::new(Some(&gio::File::for_path(path))).launch(
                        Some(this.state.window.window()),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                }
            }),
        );
    }

    pub fn open(&self, file: &gio::File) {
        let Some(path) = file.path() else {
            return;
        };
        if glib::find_program_in_path(TOOL).is_none() {
            self.state.stack.set_visible_child_name("missing");
            return;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.state.title.set_title(&name);
        self.state.title.set_subtitle(
            &path
                .parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default(),
        );
        self.state.stack.set_visible_child_name("loading");
        self.state.extract.set_sensitive(false);
        self.state.archive.replace(Some(path.clone()));
        let this = self.clone();
        glib::spawn_future_local(async move {
            let args = vec![
                TOOL.to_owned(),
                "-tf".to_owned(),
                path.to_string_lossy().into_owned(),
            ];
            match run(args).await {
                Ok(output) => {
                    let entries = parse_listing(&output);
                    this.render(&entries);
                    this.state.extract.set_sensitive(!entries.is_empty());
                    this.state.entries.replace(entries);
                }
                Err(message) => {
                    this.state.error.set_description(Some(&message));
                    this.state.stack.set_visible_child_name("error");
                }
            }
        });
    }

    fn render(&self, entries: &[String]) {
        let list = &self.state.list;
        list.remove_all();
        for entry in entries.iter().take(MAX_ROWS) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            row.append(&gtk::Image::from_icon_name(if entry.ends_with('/') {
                "folder-symbolic"
            } else {
                "text-x-generic-symbolic"
            }));
            let label = gtk::Label::new(Some(entry.trim_end_matches('/')));
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            row.append(&label);
            list.append(&row);
        }
        if entries.len() > MAX_ROWS {
            let more = gtk::Label::new(Some(
                &gettext("{count} more items")
                    .replace("{count}", &(entries.len() - MAX_ROWS).to_string()),
            ));
            more.add_css_class("dim-label");
            list.append(&more);
        }
        self.state.stack.set_visible_child_name("list");
    }

    fn choose_destination(&self) {
        let Some(archive) = self.state.archive.borrow().clone() else {
            return;
        };
        let dialog = gtk::FileDialog::new();
        dialog.set_title(&gettext("Extract to"));
        if let Some(parent) = archive.parent() {
            dialog.set_initial_folder(Some(&gio::File::for_path(parent)));
        }
        let parent = self.state.window.window().clone();
        let this = self.clone();
        dialog.select_folder(Some(&parent), gio::Cancellable::NONE, move |result| {
            if let Some(folder) = result.ok().and_then(|folder| folder.path()) {
                this.extract(&archive, &folder);
            }
        });
    }

    fn extract(&self, archive: &std::path::Path, folder: &std::path::Path) {
        let base = folder_name(
            &archive
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| gettext("Extracted")),
        );
        let mut target = folder.join(&base);
        let mut index = 2;
        while target.exists() {
            target = folder.join(format!("{base} {index}"));
            index += 1;
        }
        if let Some(bad) = self
            .state
            .entries
            .borrow()
            .iter()
            .find(|entry| !is_safe_extraction_path(&target, entry))
        {
            show_error_dialog(
                self.state.window.window(),
                &gettext("The archive is not safe to extract"),
                &gettext("It contains a path outside the destination folder: {path}")
                    .replace("{path}", bad),
            );
            return;
        }
        if let Err(error) = std::fs::create_dir_all(&target) {
            show_error_dialog(
                self.state.window.window(),
                &gettext("Could not create the folder"),
                &error.to_string(),
            );
            return;
        }
        self.state.extract.set_sensitive(false);
        let args = vec![
            TOOL.to_owned(),
            "-xf".to_owned(),
            archive.to_string_lossy().into_owned(),
            "-C".to_owned(),
            target.to_string_lossy().into_owned(),
            "--no-same-owner".to_owned(),
        ];
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = run(args).await;
            this.state.extract.set_sensitive(true);
            match result {
                Ok(_) => {
                    this.state.extracted.replace(Some(target));
                    this.state.window.show_toast_with_action(
                        &gettext("Archive extracted"),
                        &gettext("Show"),
                        "win.show-extracted",
                    );
                }
                Err(message) => show_error_dialog(
                    this.state.window.window(),
                    &gettext("Could not extract the archive"),
                    &message,
                ),
            }
        });
    }
}
