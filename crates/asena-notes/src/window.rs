//! Notes window: note list and an editor that saves as you type.
use crate::note::{file_name, is_note, matches, preview_of, title_of, MAX_BYTES};
use adw::prelude::*;
use asena_app::{prelude::*, storage};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

#[derive(Clone)]
struct Note {
    path: PathBuf,
    text: String,
    modified: i64,
}

struct State {
    window: AsenaWindow,
    split: adw::NavigationSplitView,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    content_stack: gtk::Stack,
    page: adw::NavigationPage,
    editor: gtk::TextView,
    notes: RefCell<Vec<Note>>,
    current: RefCell<Option<PathBuf>>,
    pending: RefCell<Option<glib::SourceId>>,
    loading: Cell<bool>,
}

#[derive(Clone)]
pub struct NotesWindow {
    state: Rc<State>,
}

fn directory() -> PathBuf {
    storage::data_dir("notes")
}

fn load_all() -> std::io::Result<Vec<Note>> {
    let directory = directory();
    std::fs::create_dir_all(&directory)?;
    let mut notes = Vec::new();
    for entry in std::fs::read_dir(&directory)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !is_note(&name) || !metadata.is_file() || metadata.len() > MAX_BYTES {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        notes.push(Note {
            path: entry.path(),
            text,
            modified,
        });
    }
    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(notes)
}

fn date_label(unix: i64) -> String {
    glib::DateTime::from_unix_local(unix)
        .ok()
        .and_then(|date| date.format("%e %B %Y").ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

impl NotesWindow {
    pub fn new(app: &adw::Application) -> Self {
        let split = adw::NavigationSplitView::new();
        split.set_min_sidebar_width(240.0);
        split.set_max_sidebar_width(320.0);

        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search notes")));
        search.set_margin_start(12);
        search.set_margin_end(12);
        search.set_margin_bottom(8);
        let new_button = gtk::Button::from_icon_name("list-add-symbolic");
        new_button.set_tooltip_text(Some(&gettext("New note")));
        new_button.set_action_name(Some("win.new-note"));
        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.pack_start(&new_button);
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("Open notes folder")), Some("win.open-folder"));
        menu.append(Some(&gettext("About Notes")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        sidebar_header.pack_end(&menu_button);
        let sidebar = adw::ToolbarView::new();
        sidebar.add_top_bar(&sidebar_header);
        sidebar.add_top_bar(&search);
        sidebar.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
        ));
        split.set_sidebar(Some(
            &adw::NavigationPage::builder()
                .title(gettext("Notes"))
                .tag("list")
                .child(&sidebar)
                .build(),
        ));

        let editor = gtk::TextView::new();
        editor.set_wrap_mode(gtk::WrapMode::WordChar);
        editor.set_left_margin(24);
        editor.set_right_margin(24);
        editor.set_top_margin(16);
        editor.set_bottom_margin(16);
        editor.add_css_class("asena-note-editor");
        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .child(&editor)
            .build();
        let editor_scroll = gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let empty = EmptyState::with_action(
            "accessories-notes-symbolic",
            &gettext("No notes"),
            "",
            &gettext("New note"),
            || {},
        );
        if let Some(button) = empty.child().and_downcast::<gtk::Button>() {
            button.set_action_name(Some("win.new-note"));
        }
        empty.set_description(None);
        let content_stack = gtk::Stack::new();
        content_stack.add_named(&editor_scroll, Some("editor"));
        content_stack.add_named(&empty, Some("empty"));
        let delete = gtk::Button::from_icon_name("user-trash-symbolic");
        delete.set_tooltip_text(Some(&gettext("Move to trash")));
        delete.set_action_name(Some("win.delete-note"));
        let content_header = adw::HeaderBar::new();
        content_header.pack_end(&delete);
        let content = adw::ToolbarView::new();
        content.add_top_bar(&content_header);
        content.set_content(Some(&content_stack));
        let page = adw::NavigationPage::builder()
            .title(gettext("Note"))
            .tag("note")
            .child(&content)
            .build();
        split.set_content(Some(&page));

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Notes"))
            .default_size(900, 600)
            .min_size(360, 360)
            .content(&split)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            620.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            window,
            split,
            list,
            search,
            content_stack,
            page,
            editor,
            notes: RefCell::default(),
            current: RefCell::default(),
            pending: RefCell::default(),
            loading: Cell::new(false),
        });
        let this = Self { state };
        this.setup();
        this.reload();
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
            "new-note",
            &["<Control>n"],
            Box::new(|this| this.new_note()),
        );
        add(
            "delete-note",
            &["<Control>Delete"],
            Box::new(|this| this.delete_current()),
        );
        add(
            "search",
            &["<Control>f"],
            Box::new(|this| {
                this.state.split.set_show_content(false);
                this.state.search.grab_focus();
            }),
        );
        add(
            "open-folder",
            &[],
            Box::new(|this| {
                let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(directory())));
                launcher.launch(
                    Some(this.state.window.window()),
                    gio::Cancellable::NONE,
                    |_| {},
                );
            }),
        );
        let this = self.clone();
        self.state.list.connect_row_activated(move |_, row| {
            this.select(PathBuf::from(row.widget_name().as_str()));
        });
        let this = self.clone();
        self.state
            .search
            .connect_search_changed(move |_| this.render_list());
        let this = self.clone();
        self.state
            .editor
            .buffer()
            .connect_changed(move |_| this.schedule_save());
        let this = self.clone();
        self.state.window.connect_close_request(move |_| {
            this.flush();
            glib::Propagation::Proceed
        });
    }

    fn reload(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match gio::spawn_blocking(load_all).await {
                Ok(Ok(notes)) => {
                    this.state.notes.replace(notes);
                    this.render_list();
                    let first = this
                        .state
                        .notes
                        .borrow()
                        .first()
                        .map(|note| note.path.clone());
                    match first {
                        Some(path) => this.select(path),
                        None => this.show_empty(),
                    }
                    // Start on the list when the window is narrow.
                    this.state.split.set_show_content(false);
                }
                Ok(Err(error)) => show_error_dialog(
                    this.state.window.window(),
                    &gettext("Could not read the notes"),
                    &error.to_string(),
                ),
                Err(_) => {}
            }
        });
    }

    fn render_list(&self) {
        let list = &self.state.list;
        list.remove_all();
        let query = self.state.search.text();
        let current = self.state.current.borrow().clone();
        for note in self.state.notes.borrow().iter() {
            if !matches(&note.text, &query) {
                continue;
            }
            let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
            content.set_margin_top(4);
            content.set_margin_bottom(4);
            let title = gtk::Label::new(Some(
                &title_of(&note.text).unwrap_or_else(|| gettext("Untitled note")),
            ));
            title.set_xalign(0.0);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title.add_css_class("heading");
            content.append(&title);
            let preview = preview_of(&note.text);
            let detail = gtk::Label::new(Some(&if preview.is_empty() {
                date_label(note.modified)
            } else {
                preview
            }));
            detail.set_xalign(0.0);
            detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
            detail.add_css_class("dim-label");
            detail.add_css_class("caption");
            content.append(&detail);
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&content));
            row.set_widget_name(&note.path.to_string_lossy());
            list.append(&row);
            if current.as_ref() == Some(&note.path) {
                list.select_row(Some(&row));
            }
        }
    }

    fn show_empty(&self) {
        self.state.current.replace(None);
        self.state.content_stack.set_visible_child_name("empty");
        self.state.page.set_title(&gettext("Notes"));
        if let Some(action) = self
            .state
            .window
            .lookup_action("delete-note")
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(false);
        }
    }

    fn select(&self, path: PathBuf) {
        self.flush();
        let text = self
            .state
            .notes
            .borrow()
            .iter()
            .find(|note| note.path == path)
            .map(|note| note.text.clone());
        let Some(text) = text else {
            return;
        };
        self.state.loading.set(true);
        self.state.editor.buffer().set_text(&text);
        self.state.loading.set(false);
        self.state.current.replace(Some(path));
        self.state.content_stack.set_visible_child_name("editor");
        self.state
            .page
            .set_title(&title_of(&text).unwrap_or_else(|| gettext("Untitled note")));
        if let Some(action) = self
            .state
            .window
            .lookup_action("delete-note")
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(true);
        }
        self.render_list();
        self.state.split.set_show_content(true);
        self.state.editor.grab_focus();
    }

    fn new_note(&self) {
        self.flush();
        let path = directory().join(file_name(glib::real_time()));
        let now = glib::real_time() / 1_000_000;
        self.state.notes.borrow_mut().insert(
            0,
            Note {
                path: path.clone(),
                text: String::new(),
                modified: now,
            },
        );
        self.state.search.set_text("");
        self.select(path);
    }

    fn schedule_save(&self) {
        if self.state.loading.get() {
            return;
        }
        let Some(path) = self.state.current.borrow().clone() else {
            return;
        };
        let buffer = self.state.editor.buffer();
        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();
        if let Some(note) = self
            .state
            .notes
            .borrow_mut()
            .iter_mut()
            .find(|note| note.path == path)
        {
            note.text = text.clone();
            note.modified = glib::real_time() / 1_000_000;
        }
        self.state
            .page
            .set_title(&title_of(&text).unwrap_or_else(|| gettext("Untitled note")));
        if let Some(source) = self.state.pending.borrow_mut().take() {
            source.remove();
        }
        let this = self.clone();
        let source = glib::timeout_add_local_once(Duration::from_millis(700), move || {
            this.state.pending.borrow_mut().take();
            this.write(&path, text);
            this.render_list();
        });
        self.state.pending.replace(Some(source));
    }

    /// Writes a pending change immediately, for example before switching notes.
    fn flush(&self) {
        let Some(source) = self.state.pending.borrow_mut().take() else {
            return;
        };
        source.remove();
        let Some(path) = self.state.current.borrow().clone() else {
            return;
        };
        let text = self
            .state
            .notes
            .borrow()
            .iter()
            .find(|note| note.path == path)
            .map(|note| note.text.clone());
        if let Some(text) = text {
            self.write(&path, text);
        }
    }

    fn write(&self, path: &std::path::Path, text: String) {
        let path = path.to_path_buf();
        let window = self.state.window.clone();
        glib::spawn_future_local(async move {
            let result =
                gio::spawn_blocking(move || storage::write_atomic(&path, text.as_bytes())).await;
            if let Ok(Err(error)) = result {
                window.show_toast(
                    &gettext("Could not save the note: {error}")
                        .replace("{error}", &error.to_string()),
                );
            }
        });
    }

    fn delete_current(&self) {
        if let Some(source) = self.state.pending.borrow_mut().take() {
            source.remove();
        }
        let Some(path) = self.state.current.borrow_mut().take() else {
            return;
        };
        self.state
            .notes
            .borrow_mut()
            .retain(|note| note.path != path);
        let file = gio::File::for_path(&path);
        let window = self.state.window.clone();
        let exists = path.exists();
        if exists {
            file.trash_async(
                glib::Priority::DEFAULT,
                gio::Cancellable::NONE,
                move |result| match result {
                    Ok(()) => window.show_toast(&gettext("Note moved to the trash")),
                    Err(error) => show_error_dialog(
                        window.window(),
                        &gettext("Could not delete the note"),
                        error.message(),
                    ),
                },
            );
        }
        let next = self
            .state
            .notes
            .borrow()
            .first()
            .map(|note| note.path.clone());
        self.render_list();
        match next {
            Some(path) => self.select(path),
            None => self.show_empty(),
        }
    }
}
