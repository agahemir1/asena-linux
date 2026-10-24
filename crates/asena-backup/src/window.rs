//! Backup window: the backup drive, backing up now, and the list of backups.
use crate::backup::{self, Outcome};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
};

const SCHEMA: &str = "org.asenalinux.backup";

pub fn stamp() -> String {
    glib::DateTime::now_local()
        .and_then(|now| now.format("%Y-%m-%d %H-%M-%S"))
        .map(|text| text.to_string())
        .unwrap_or_default()
}

/// `2026-10-23 14-05-33` → a localized date and time.
fn backup_label(name: &str) -> String {
    let numbers: Vec<i32> = name
        .split(['-', ' '])
        .filter_map(|part| part.parse().ok())
        .collect();
    let [year, month, day, hour, minute, second] = numbers[..] else {
        return name.to_owned();
    };
    glib::DateTime::from_local(year, month, day, hour, minute, second as f64)
        .and_then(|time| time.format("%e %B %Y, %H:%M"))
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|_| name.to_owned())
}

fn time_label(unix: i64) -> String {
    if unix <= 0 {
        return gettext("Never");
    }
    glib::DateTime::from_unix_local(unix)
        .and_then(|time| time.format("%e %B %Y, %H:%M"))
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

/// What the window learns about the destination off the main thread.
struct Drive {
    connected: bool,
    history: bool,
    backups: Vec<String>,
}

fn inspect(destination: &Path) -> Drive {
    if !destination.is_dir() {
        return Drive {
            connected: false,
            history: false,
            backups: Vec::new(),
        };
    }
    let filesystem = gio::File::for_path(destination)
        .query_filesystem_info("filesystem::type", gio::Cancellable::NONE)
        .ok()
        .and_then(|info| info.attribute_string("filesystem::type"))
        .map(|kind| kind.to_string())
        .unwrap_or_default();
    Drive {
        connected: true,
        history: backup::keeps_history(&filesystem),
        backups: backup::list(&folder(destination)),
    }
}

pub fn folder(destination: &Path) -> PathBuf {
    backup::machine_folder(
        destination,
        &glib::user_name().to_string_lossy(),
        &glib::host_name(),
    )
}

/// A backup on the disk it protects would be lost with that disk.
pub fn same_disk(destination: &Path) -> bool {
    match (
        std::fs::metadata(destination),
        std::fs::metadata(glib::home_dir()),
    ) {
        (Ok(destination), Ok(home)) => destination.dev() == home.dev(),
        _ => false,
    }
}

enum Message {
    Started(u32),
    Progress(f64),
    Finished(Outcome),
}

struct State {
    window: AsenaWindow,
    settings: gio::Settings,
    stack: gtk::Stack,
    banner: adw::Banner,
    destination: adw::ActionRow,
    last: gtk::Label,
    start: gtk::Button,
    stop: gtk::Button,
    progress: gtk::ProgressBar,
    backups: adw::PreferencesGroup,
    rows: RefCell<Vec<adw::ActionRow>>,
    pid: Cell<Option<u32>>,
    running: Cell<bool>,
}

#[derive(Clone)]
pub struct BackupWindow {
    state: Rc<State>,
}

impl BackupWindow {
    pub fn new(app: &adw::Application) -> Option<Self> {
        let settings = asena_core::settings::load(SCHEMA).ok()?;
        let empty = adw::StatusPage::builder()
            .icon_name("drive-removable-media-symbolic")
            .title(gettext("No backup drive"))
            .description(gettext(
                "Connect an external drive and choose a folder on it for the backups.",
            ))
            .build();
        let choose = gtk::Button::with_label(&gettext("Choose folder"));
        choose.add_css_class("pill");
        choose.add_css_class("suggested-action");
        choose.set_halign(gtk::Align::Center);
        choose.set_action_name(Some("win.choose"));
        empty.set_child(Some(&choose));

        let page = adw::PreferencesPage::new();
        let main_group = adw::PreferencesGroup::new();
        let destination = adw::ActionRow::builder()
            .title(gettext("Backup folder"))
            .build();
        let change = gtk::Button::with_label(&gettext("Change"));
        change.set_valign(gtk::Align::Center);
        change.set_action_name(Some("win.choose"));
        destination.add_suffix(&change);
        main_group.add(&destination);
        let last_row = adw::ActionRow::builder()
            .title(gettext("Last backup"))
            .build();
        let last = gtk::Label::new(None);
        last.add_css_class("dim-label");
        last_row.add_suffix(&last);
        main_group.add(&last_row);
        let automatic = adw::SwitchRow::builder()
            .title(gettext("Back up daily"))
            .subtitle(gettext("While the backup drive is connected"))
            .build();
        settings.bind("automatic", &automatic, "active").build();
        main_group.add(&automatic);
        page.add(&main_group);

        let start = gtk::Button::with_label(&gettext("Back up now"));
        start.add_css_class("pill");
        start.add_css_class("suggested-action");
        start.set_action_name(Some("win.start"));
        let stop = gtk::Button::with_label(&gettext("Stop"));
        stop.add_css_class("pill");
        stop.set_action_name(Some("win.stop"));
        stop.set_visible(false);
        let progress = gtk::ProgressBar::new();
        progress.set_visible(false);
        progress.set_valign(gtk::Align::Center);
        progress.set_hexpand(true);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        controls.set_halign(gtk::Align::Fill);
        controls.append(&progress);
        controls.append(&start);
        controls.append(&stop);
        let controls_group = adw::PreferencesGroup::new();
        controls_group.add(&controls);
        page.add(&controls_group);

        let backups = adw::PreferencesGroup::builder()
            .title(gettext("Backups"))
            .build();
        backups.set_visible(false);
        page.add(&backups);

        let banner = adw::Banner::new(&gettext("The backup drive is not connected"));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&banner);
        content.append(&page);
        page.set_vexpand(true);
        let stack = gtk::Stack::new();
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&content, Some("main"));

        let menu = gio::Menu::new();
        menu.append(Some(&gettext("About Backup")), Some("app.about"));
        let header = AsenaHeaderBarBuilder::new()
            .title(gettext("Backup"))
            .primary_menu(menu.upcast_ref())
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Backup"))
            .default_size(640, 620)
            .min_size(360, 420)
            .content(&toolbar)
            .build();
        let state = Rc::new(State {
            window,
            settings,
            stack,
            banner,
            destination,
            last,
            start,
            stop,
            progress,
            backups,
            rows: RefCell::default(),
            pid: Cell::new(None),
            running: Cell::new(false),
        });
        let this = Self { state };
        this.install_actions();
        let cloned = this.clone();
        this.state
            .settings
            .connect_changed(None, move |_, _| cloned.refresh());
        // Drives come and go; look again when the window gets focus.
        let cloned = this.clone();
        this.state.window.connect_is_active_notify(move |window| {
            if window.is_active() {
                cloned.refresh();
            }
        });
        this.refresh();
        Some(this)
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn destination(&self) -> Option<PathBuf> {
        let text = self.state.settings.string("destination");
        (!text.is_empty()).then(|| PathBuf::from(text.as_str()))
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let add = |name: &str, run: fn(&Self)| {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| run(&this));
            window.add_action(&action);
        };
        add("choose", |this| this.choose());
        add("start", |this| this.start());
        add("stop", |this| {
            if let Some(pid) = this.state.pid.get() {
                glib::spawn_future_local(async move {
                    let _ = gio::spawn_blocking(move || {
                        std::process::Command::new("kill")
                            .args(["-TERM", "--", &pid.to_string()])
                            .status()
                    })
                    .await;
                });
            }
        });
    }

    fn refresh(&self) {
        let Some(destination) = self.destination() else {
            self.state.stack.set_visible_child_name("empty");
            return;
        };
        self.state.stack.set_visible_child_name("main");
        self.state
            .destination
            .set_subtitle(&glib::markup_escape_text(&destination.to_string_lossy()));
        self.state
            .last
            .set_text(&time_label(self.state.settings.int64("last-backup")));
        let this = self.clone();
        glib::spawn_future_local(async move {
            let lookup = destination.clone();
            let Ok(drive) = gio::spawn_blocking(move || inspect(&lookup)).await else {
                return;
            };
            let state = &this.state;
            state.banner.set_revealed(!drive.connected);
            state
                .start
                .set_sensitive(drive.connected && !state.running.get());
            for row in state.rows.borrow_mut().drain(..) {
                state.backups.remove(&row);
            }
            if drive.connected && !drive.history {
                state.destination.set_subtitle(&glib::markup_escape_text(&format!(
                    "{}\n{}",
                    destination.to_string_lossy(),
                    gettext("This drive keeps only the newest backup. Drives formatted for Linux keep earlier ones too.")
                )));
            }
            let folder = folder(&destination);
            for name in &drive.backups {
                let row = adw::ActionRow::builder().title(backup_label(name)).build();
                let open = gtk::Button::from_icon_name("folder-open-symbolic");
                open.add_css_class("flat");
                open.set_valign(gtk::Align::Center);
                open.set_tooltip_text(Some(&gettext("Open")));
                let path = folder.join(name);
                let window = state.window.window().clone();
                let target = path.clone();
                open.connect_clicked(move |_| {
                    gtk::FileLauncher::new(Some(&gio::File::for_path(&target))).launch(
                        Some(&window),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                });
                let restore = gtk::Button::with_label(&gettext("Restore"));
                restore.set_valign(gtk::Align::Center);
                let cloned = this.clone();
                let label = backup_label(name);
                restore.connect_clicked(move |_| cloned.confirm_restore(path.clone(), &label));
                row.add_suffix(&open);
                row.add_suffix(&restore);
                state.backups.add(&row);
                state.rows.borrow_mut().push(row);
            }
            state.backups.set_visible(!drive.backups.is_empty());
        });
    }

    fn choose(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Choose a folder for backups"))
            .build();
        let this = self.clone();
        dialog.select_folder(
            Some(self.state.window.window()),
            gio::Cancellable::NONE,
            move |result| {
                let Some(path) = result.ok().and_then(|file| file.path()) else {
                    return;
                };
                let this = this.clone();
                glib::spawn_future_local(async move {
                    let check = path.clone();
                    let same = gio::spawn_blocking(move || same_disk(&check))
                        .await
                        .unwrap_or(false);
                    if same {
                        this.state
                            .window
                            .show_toast(&gettext("Choose a folder on another drive"));
                        return;
                    }
                    if let Err(error) = this
                        .state
                        .settings
                        .set_string("destination", &path.to_string_lossy())
                    {
                        tracing::warn!(%error, "Could not save the backup folder");
                    }
                });
            },
        );
    }

    fn start(&self) {
        let Some(destination) = self.destination() else {
            return;
        };
        if self.state.running.replace(true) {
            return;
        }
        let state = &self.state;
        state.start.set_visible(false);
        state.stop.set_visible(true);
        state.progress.set_fraction(0.0);
        state.progress.set_visible(true);
        let (sender, receiver) = async_channel::unbounded();
        gio::spawn_blocking(move || {
            let history = inspect(&destination).history;
            let started = sender.clone();
            let progress = sender.clone();
            let outcome = backup::run(
                &glib::home_dir(),
                &folder(&destination),
                &stamp(),
                history,
                move |child| {
                    let _ = started.send_blocking(Message::Started(child.id()));
                },
                move |fraction| {
                    let _ = progress.send_blocking(Message::Progress(fraction));
                },
            );
            let _ = sender.send_blocking(Message::Finished(outcome));
        });
        let this = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(message) = receiver.recv().await {
                match message {
                    Message::Started(pid) => this.state.pid.set(Some(pid)),
                    Message::Progress(fraction) => this.state.progress.set_fraction(fraction),
                    Message::Finished(outcome) => {
                        this.finish(outcome);
                        break;
                    }
                }
            }
        });
    }

    fn finish(&self, outcome: Outcome) {
        let state = &self.state;
        state.running.set(false);
        state.pid.set(None);
        state.start.set_visible(true);
        state.stop.set_visible(false);
        state.progress.set_visible(false);
        match outcome {
            Outcome::Done => {
                let now = glib::DateTime::now_utc()
                    .map(|now| now.to_unix())
                    .unwrap_or_default();
                let _ = state.settings.set_int64("last-backup", now);
                state.window.show_toast(&gettext("Backup complete"));
            }
            Outcome::Stopped => state.window.show_toast(&gettext("Backup stopped")),
            Outcome::Failed(error) => {
                tracing::warn!(%error, "Backup failed");
                show_error_dialog(
                    state.window.window(),
                    &gettext("Backup failed"),
                    &gettext("Check that the backup drive is connected and has free space."),
                );
            }
        }
        self.refresh();
    }

    fn confirm_restore(&self, backup: PathBuf, label: &str) {
        let this = self.clone();
        confirm_dialog(
            self.state.window.window(),
            &gettext("Restore files from {date}?").replace("{date}", label),
            &gettext("Files in your home folder that changed since this backup are replaced with the backed up copies. Files added since then stay."),
            &gettext("Restore"),
            true,
            move || {
                let this = this.clone();
                glib::spawn_future_local(async move {
                    let result =
                        gio::spawn_blocking(move || backup::restore(&backup, &glib::home_dir()))
                            .await
                            .unwrap_or_else(|_| Err("stopped".into()));
                    match result {
                        Ok(()) => this.state.window.show_toast(&gettext("Files restored")),
                        Err(error) => {
                            tracing::warn!(%error, "Restore failed");
                            this.state
                                .window
                                .show_toast(&gettext("Could not restore the files"));
                        }
                    }
                });
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_fall_back_to_the_name() {
        assert_eq!(backup_label("broken"), "broken");
        assert!(!backup_label("2026-10-23 14-05-33").is_empty());
        assert_eq!(time_label(0), "Never");
    }
}
