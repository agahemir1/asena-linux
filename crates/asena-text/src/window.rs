//! Tabbed plain text editor window.
use crate::document::{decode, tab_title, DecodeError};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Doc {
    page: adw::TabPage,
    view: gtk::TextView,
    file: RefCell<Option<gio::File>>,
}

impl Doc {
    fn buffer(&self) -> gtk::TextBuffer {
        self.view.buffer()
    }

    fn name(&self) -> Option<String> {
        self.file
            .borrow()
            .as_ref()
            .and_then(|file| file.basename())
            .map(|name| name.to_string_lossy().into_owned())
    }

    fn refresh_title(&self) {
        let title = tab_title(
            self.name().as_deref(),
            &gettext("Untitled"),
            self.buffer().is_modified(),
        );
        self.page.set_title(&title);
        let tooltip = self
            .file
            .borrow()
            .as_ref()
            .map(|file| file.parse_name().to_string())
            .unwrap_or_default();
        self.page.set_tooltip(&tooltip);
    }

    fn is_empty_untitled(&self) -> bool {
        self.file.borrow().is_none()
            && !self.buffer().is_modified()
            && self.buffer().char_count() == 0
    }
}

struct State {
    window: AsenaWindow,
    tabs: adw::TabView,
    docs: RefCell<Vec<Rc<Doc>>>,
    search_bar: gtk::SearchBar,
    search: gtk::SearchEntry,
    position: gtk::Label,
    wrap: Cell<bool>,
    closing: Cell<bool>,
}

#[derive(Clone)]
pub struct TextWindow {
    state: Rc<State>,
}

impl TextWindow {
    pub fn new(app: &adw::Application) -> Self {
        let tabs = adw::TabView::new();
        let tab_bar = adw::TabBar::new();
        tab_bar.set_view(Some(&tabs));
        tab_bar.set_autohide(true);

        let header = adw::HeaderBar::new();
        let open = gtk::Button::with_label(&gettext("Open"));
        open.set_action_name(Some("win.open"));
        header.pack_start(&open);
        let new = gtk::Button::from_icon_name("tab-new-symbolic");
        new.set_tooltip_text(Some(&gettext("New document")));
        new.set_action_name(Some("win.new"));
        header.pack_start(&new);
        let menu = gio::Menu::new();
        let file = gio::Menu::new();
        file.append(Some(&gettext("Save as…")), Some("win.save-as"));
        file.append(Some(&gettext("Find")), Some("win.find"));
        menu.append_section(None, &file);
        let view = gio::Menu::new();
        view.append(Some(&gettext("Wrap lines")), Some("win.wrap"));
        menu.append_section(None, &view);
        let other = gio::Menu::new();
        other.append(Some(&gettext("About Text Editor")), Some("app.about"));
        menu.append_section(None, &other);
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let save = gtk::Button::with_label(&gettext("Save"));
        save.set_action_name(Some("win.save"));
        header.pack_end(&save);

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Find in document")));
        search.set_hexpand(true);
        let previous = gtk::Button::from_icon_name("go-up-symbolic");
        previous.set_tooltip_text(Some(&gettext("Previous match")));
        let next = gtk::Button::from_icon_name("go-down-symbolic");
        next.set_tooltip_text(Some(&gettext("Next match")));
        let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        search_box.append(&search);
        search_box.append(&previous);
        search_box.append(&next);
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(
            &adw::Clamp::builder()
                .child(&search_box)
                .maximum_size(560)
                .build(),
        ));
        search_bar.connect_entry(&search);

        let position = gtk::Label::new(None);
        position.add_css_class("dim-label");
        position.add_css_class("caption");
        position.add_css_class("numeric");
        let status = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        status.set_margin_start(12);
        status.set_margin_end(12);
        status.set_margin_top(4);
        status.set_margin_bottom(4);
        position.set_hexpand(true);
        position.set_xalign(1.0);
        status.append(&position);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&tab_bar);
        toolbar.add_top_bar(&search_bar);
        toolbar.set_content(Some(&tabs));
        toolbar.add_bottom_bar(&status);

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Text Editor"))
            .default_size(820, 600)
            .min_size(360, 300)
            .content(&toolbar)
            .build();

        let state = Rc::new(State {
            window,
            tabs,
            docs: RefCell::default(),
            search_bar,
            search,
            position,
            wrap: Cell::new(true),
            closing: Cell::new(false),
        });
        let this = Self { state };
        this.setup_actions();
        this.setup_tabs();
        this.setup_search(&previous, &next);
        this.new_document(None, "");
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    pub fn open_files(&self, files: &[gio::File]) {
        for file in files {
            self.load(file.clone());
        }
    }

    fn current(&self) -> Option<Rc<Doc>> {
        let page = self.state.tabs.selected_page()?;
        self.doc_for(&page)
    }

    fn doc_for(&self, page: &adw::TabPage) -> Option<Rc<Doc>> {
        self.state
            .docs
            .borrow()
            .iter()
            .find(|doc| &doc.page == page)
            .cloned()
    }

    fn new_document(&self, file: Option<gio::File>, text: &str) -> Rc<Doc> {
        let view = gtk::TextView::new();
        view.set_monospace(true);
        view.set_left_margin(12);
        view.set_right_margin(12);
        view.set_top_margin(8);
        view.set_bottom_margin(8);
        view.set_wrap_mode(if self.state.wrap.get() {
            gtk::WrapMode::WordChar
        } else {
            gtk::WrapMode::None
        });
        let buffer = view.buffer();
        buffer.set_text(text);
        buffer.set_modified(false);
        buffer.place_cursor(&buffer.start_iter());
        let scroll = gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .hexpand(true)
            .build();
        let page = self.state.tabs.append(&scroll);
        let doc = Rc::new(Doc {
            page,
            view,
            file: RefCell::new(file),
        });
        doc.refresh_title();
        let weak = Rc::downgrade(&doc);
        buffer.connect_modified_changed(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.refresh_title();
            }
        });
        let this = self.clone();
        buffer.connect_mark_set(move |buffer, _, mark| {
            if mark.name().as_deref() == Some("insert") {
                this.update_position(buffer);
            }
        });
        self.state.docs.borrow_mut().push(doc.clone());
        self.state.tabs.set_selected_page(&doc.page);
        doc.view.grab_focus();
        doc
    }

    fn update_position(&self, buffer: &gtk::TextBuffer) {
        let iter = buffer.iter_at_mark(&buffer.get_insert());
        self.state.position.set_text(
            &gettext("Line {line}, column {column}")
                .replace("{line}", &(iter.line() + 1).to_string())
                .replace("{column}", &(iter.line_offset() + 1).to_string()),
        );
    }

    fn load(&self, file: gio::File) {
        if let Some(existing) = self.state.docs.borrow().iter().find(|doc| {
            doc.file
                .borrow()
                .as_ref()
                .is_some_and(|open| open.equal(&file))
        }) {
            self.state.tabs.set_selected_page(&existing.page);
            return;
        }
        let this = self.clone();
        file.clone()
            .load_contents_async(gio::Cancellable::NONE, move |result| {
                let decoded = match result {
                    Ok((bytes, _)) => decode(&bytes).map_err(|error| match error {
                        DecodeError::TooLarge => gettext("The file is too large to edit."),
                        DecodeError::NotText => {
                            gettext("The file is not UTF-8 text and cannot be edited here.")
                        }
                    }),
                    Err(error) => Err(error.message().to_owned()),
                };
                match decoded {
                    Ok(text) => {
                        // Replace the initial empty document instead of keeping it.
                        let reuse = this.current().filter(|doc| doc.is_empty_untitled());
                        match reuse {
                            Some(doc) => {
                                let buffer = doc.buffer();
                                buffer.set_text(&text);
                                buffer.set_modified(false);
                                buffer.place_cursor(&buffer.start_iter());
                                doc.file.replace(Some(file));
                                doc.refresh_title();
                            }
                            None => {
                                this.new_document(Some(file), &text);
                            }
                        }
                    }
                    Err(message) => show_error_dialog(
                        this.state.window.window(),
                        &gettext("Could not open the file"),
                        &message,
                    ),
                }
            });
    }

    fn save(&self, doc: &Rc<Doc>, done: Rc<dyn Fn(bool)>) {
        let Some(file) = doc.file.borrow().clone() else {
            self.save_as(doc, done);
            return;
        };
        let buffer = doc.buffer();
        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string();
        let window = self.state.window.clone();
        let doc = doc.clone();
        file.replace_contents_async(
            text.into_bytes(),
            None,
            false,
            gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::Cancellable::NONE,
            move |result| match result {
                Ok(_) => {
                    doc.buffer().set_modified(false);
                    doc.refresh_title();
                    done(true);
                }
                Err((_, error)) => {
                    show_error_dialog(
                        window.window(),
                        &gettext("Could not save the file"),
                        error.message(),
                    );
                    done(false);
                }
            },
        );
    }

    fn save_as(&self, doc: &Rc<Doc>, done: Rc<dyn Fn(bool)>) {
        let dialog = gtk::FileDialog::new();
        dialog.set_initial_name(Some(
            &doc.name().unwrap_or_else(|| gettext("Untitled") + ".txt"),
        ));
        let this = self.clone();
        let doc = doc.clone();
        dialog.save(
            Some(self.state.window.window()),
            gio::Cancellable::NONE,
            move |result| match result {
                Ok(file) => {
                    doc.file.replace(Some(file));
                    this.save(&doc, done.clone());
                }
                Err(_) => done(false),
            },
        );
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
            "new",
            &["<Control>n", "<Control>t"],
            Box::new(|this| {
                this.new_document(None, "");
            }),
        );
        add(
            "open",
            &["<Control>o"],
            Box::new(|this| {
                let dialog = gtk::FileDialog::new();
                let parent = this.state.window.window().clone();
                let this = this.clone();
                dialog.open_multiple(Some(&parent), gio::Cancellable::NONE, move |result| {
                    if let Ok(files) = result {
                        let files: Vec<gio::File> = (0..files.n_items())
                            .filter_map(|index| files.item(index).and_downcast::<gio::File>())
                            .collect();
                        this.open_files(&files);
                    }
                });
            }),
        );
        add(
            "save",
            &["<Control>s"],
            Box::new(|this| {
                if let Some(doc) = this.current() {
                    this.save(&doc, Rc::new(|_| {}));
                }
            }),
        );
        add(
            "save-as",
            &["<Control><Shift>s"],
            Box::new(|this| {
                if let Some(doc) = this.current() {
                    this.save_as(&doc, Rc::new(|_| {}));
                }
            }),
        );
        add(
            "close-tab",
            &["<Control>w"],
            Box::new(|this| {
                if let Some(page) = this.state.tabs.selected_page() {
                    this.state.tabs.close_page(&page);
                }
            }),
        );
        add(
            "find",
            &["<Control>f"],
            Box::new(|this| {
                this.state.search_bar.set_search_mode(true);
                this.state.search.grab_focus();
            }),
        );
        let wrap = gio::SimpleAction::new_stateful("wrap", None, &true.to_variant());
        let this = self.clone();
        wrap.connect_activate(move |action, _| {
            let value = !this.state.wrap.get();
            this.state.wrap.set(value);
            action.set_state(&value.to_variant());
            for doc in this.state.docs.borrow().iter() {
                doc.view.set_wrap_mode(if value {
                    gtk::WrapMode::WordChar
                } else {
                    gtk::WrapMode::None
                });
            }
        });
        window.add_action(&wrap);
    }

    fn setup_tabs(&self) {
        let this = self.clone();
        self.state.tabs.connect_close_page(move |tabs, page| {
            let Some(doc) = this.doc_for(page) else {
                tabs.close_page_finish(page, true);
                return glib::Propagation::Stop;
            };
            if !doc.buffer().is_modified() {
                this.state
                    .docs
                    .borrow_mut()
                    .retain(|item| !Rc::ptr_eq(item, &doc));
                tabs.close_page_finish(page, true);
                if tabs.n_pages() == 0 && !this.state.closing.get() {
                    this.new_document(None, "");
                }
                return glib::Propagation::Stop;
            }
            let dialog = adw::AlertDialog::new(
                Some(&gettext("Save changes?")),
                Some(&gettext(
                    "Unsaved changes will be lost if you close without saving.",
                )),
            );
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("discard", &gettext("Close without saving"));
            dialog.add_response("save", &gettext("Save"));
            dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("save"));
            dialog.set_close_response("cancel");
            let parent = this.state.window.window().clone();
            let this = this.clone();
            let page = page.clone();
            dialog.connect_response(None, move |_, response| {
                let finish = {
                    let this = this.clone();
                    let page = page.clone();
                    let doc = doc.clone();
                    Rc::new(move |close: bool| {
                        if close {
                            this.state
                                .docs
                                .borrow_mut()
                                .retain(|item| !Rc::ptr_eq(item, &doc));
                        }
                        this.state.tabs.close_page_finish(&page, close);
                        if close && this.state.tabs.n_pages() == 0 && !this.state.closing.get() {
                            this.new_document(None, "");
                        }
                    })
                };
                match response {
                    "save" => this.save(&doc, finish),
                    "discard" => finish(true),
                    _ => finish(false),
                }
            });
            dialog.present(Some(&parent));
            glib::Propagation::Stop
        });
        let this = self.clone();
        self.state.tabs.connect_selected_page_notify(move |_| {
            if let Some(doc) = this.current() {
                this.update_position(&doc.buffer());
            }
        });
        let this = self.clone();
        self.state.window.connect_close_request(move |window| {
            let unsaved = this
                .state
                .docs
                .borrow()
                .iter()
                .any(|doc| doc.buffer().is_modified());
            if !unsaved || this.state.closing.get() {
                return glib::Propagation::Proceed;
            }
            let this = this.clone();
            let parent = window.clone();
            let window = window.clone();
            confirm_dialog(
                &parent,
                &gettext("Close without saving?"),
                &gettext("Some documents have unsaved changes."),
                &gettext("Close without saving"),
                true,
                move || {
                    this.state.closing.set(true);
                    window.close();
                },
            );
            glib::Propagation::Stop
        });
    }

    fn setup_search(&self, previous: &gtk::Button, next: &gtk::Button) {
        let this = self.clone();
        self.state.search.connect_activate(move |_| this.find(true));
        let this = self.clone();
        self.state
            .search
            .connect_search_changed(move |_| this.find_from_start());
        let this = self.clone();
        next.connect_clicked(move |_| this.find(true));
        let this = self.clone();
        previous.connect_clicked(move |_| this.find(false));
        let this = self.clone();
        self.state
            .search
            .connect_previous_match(move |_| this.find(false));
        let this = self.clone();
        self.state
            .search
            .connect_next_match(move |_| this.find(true));
    }

    fn find_from_start(&self) {
        if let Some(doc) = self.current() {
            let buffer = doc.buffer();
            buffer.place_cursor(&buffer.start_iter());
        }
        self.find(true);
    }

    fn find(&self, forward: bool) {
        let Some(doc) = self.current() else {
            return;
        };
        let query = self.state.search.text();
        self.state.search.remove_css_class("error");
        if query.is_empty() {
            return;
        }
        let buffer = doc.buffer();
        let flags = gtk::TextSearchFlags::CASE_INSENSITIVE | gtk::TextSearchFlags::TEXT_ONLY;
        let (start, end) = buffer.selection_bounds().unwrap_or_else(|| {
            let cursor = buffer.iter_at_mark(&buffer.get_insert());
            (cursor, cursor)
        });
        let found = if forward {
            end.forward_search(&query, flags, None)
                .or_else(|| buffer.start_iter().forward_search(&query, flags, None))
        } else {
            start
                .backward_search(&query, flags, None)
                .or_else(|| buffer.end_iter().backward_search(&query, flags, None))
        };
        match found {
            Some((mut match_start, match_end)) => {
                buffer.select_range(&match_start, &match_end);
                doc.view
                    .scroll_to_iter(&mut match_start, 0.2, false, 0.0, 0.0);
            }
            None => self.state.search.add_css_class("error"),
        }
    }
}
