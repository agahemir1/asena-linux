//! Terminal window: tabs of VTE terminals running the user's shell.
use adw::prelude::*;
use asena_app::prelude::*;
use std::{cell::Cell, rc::Rc};
use vte4::prelude::*;

/// PCRE2_CASELESS | PCRE2_MULTILINE; VTE needs multiline search patterns.
const SEARCH_FLAGS: u32 = 0x0000_0008 | 0x0000_0400;
const ZOOM_STEP: f64 = 0.1;

/// `file:///home/a/b` → `/home/a/b`, used to open new tabs in the same folder.
pub fn directory_of(uri: &str) -> Option<String> {
    gio::File::for_uri(uri)
        .path()
        .map(|path| path.to_string_lossy().into_owned())
}

/// Each tab holds a scrolled window around its terminal.
fn terminal_of(page: &adw::TabPage) -> Option<vte4::Terminal> {
    page.child()
        .downcast::<gtk::ScrolledWindow>()
        .ok()?
        .child()?
        .downcast::<vte4::Terminal>()
        .ok()
}

struct State {
    window: AsenaWindow,
    tabs: adw::TabView,
    search_bar: gtk::SearchBar,
    search: gtk::SearchEntry,
    zoom: Cell<f64>,
}

#[derive(Clone)]
pub struct TerminalWindow {
    state: Rc<State>,
}

impl TerminalWindow {
    pub fn new(app: &adw::Application) -> Self {
        Self::with_directory(app, None)
    }

    pub fn with_directory(app: &adw::Application, directory: Option<String>) -> Self {
        let tabs = adw::TabView::new();
        let tab_bar = adw::TabBar::new();
        tab_bar.set_view(Some(&tabs));
        let new_tab = gtk::Button::from_icon_name("tab-new-symbolic");
        new_tab.set_tooltip_text(Some(&gettext("New tab")));
        new_tab.set_action_name(Some("win.new-tab"));
        let find = gtk::ToggleButton::new();
        let menu = gio::Menu::new();
        let edit = gio::Menu::new();
        edit.append(Some(&gettext("Copy")), Some("win.copy"));
        edit.append(Some(&gettext("Paste")), Some("win.paste"));
        edit.append(Some(&gettext("Select all")), Some("win.select-all"));
        menu.append_section(None, &edit);
        let view = gio::Menu::new();
        view.append(Some(&gettext("Zoom in")), Some("win.zoom-in"));
        view.append(Some(&gettext("Zoom out")), Some("win.zoom-out"));
        view.append(Some(&gettext("Original size")), Some("win.zoom-reset"));
        menu.append_section(None, &view);
        menu.append(Some(&gettext("About Terminal")), Some("app.about"));
        let header = AsenaHeaderBarBuilder::new()
            .title(gettext("Terminal"))
            .pack_start(&new_tab)
            .search_toggle(&find)
            .primary_menu(menu.upcast_ref())
            .build();
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Find")));
        let previous = gtk::Button::from_icon_name("go-up-symbolic");
        previous.set_tooltip_text(Some(&gettext("Previous match")));
        previous.set_action_name(Some("win.find-previous"));
        let next = gtk::Button::from_icon_name("go-down-symbolic");
        next.set_tooltip_text(Some(&gettext("Next match")));
        next.set_action_name(Some("win.find-next"));
        let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        search_box.append(&search);
        search_box.append(&previous);
        search_box.append(&next);
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(&search_box));
        search_bar.connect_entry(&search);
        find.bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&tab_bar);
        toolbar.add_top_bar(&search_bar);
        toolbar.set_content(Some(&tabs));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Terminal"))
            .default_size(800, 500)
            .min_size(360, 240)
            .content(&toolbar)
            .build();
        let state = Rc::new(State {
            window,
            tabs,
            search_bar,
            search,
            zoom: Cell::new(1.0),
        });
        let this = Self { state };
        this.install_actions();
        let cloned = this.clone();
        this.state.tabs.connect_close_page(move |tabs, page| {
            tabs.close_page_finish(page, true);
            if tabs.n_pages() == 0 {
                cloned.state.window.close();
            }
            glib::Propagation::Stop
        });
        let cloned = this.clone();
        this.state.tabs.connect_selected_page_notify(move |_| {
            if let Some(terminal) = cloned.current() {
                terminal.grab_focus();
                cloned
                    .state
                    .window
                    .set_title(Some(&cloned.title_of(&terminal)));
            }
        });
        let cloned = this.clone();
        this.state
            .search
            .connect_search_changed(move |entry| cloned.set_search(&entry.text()));
        let cloned = this.clone();
        this.state
            .search
            .connect_activate(move |_| cloned.find(true));
        let cloned = this.clone();
        this.state.search.connect_stop_search(move |_| {
            cloned.state.search_bar.set_search_mode(false);
            if let Some(terminal) = cloned.current() {
                terminal.grab_focus();
            }
        });
        this.open_tab(directory);
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn current(&self) -> Option<vte4::Terminal> {
        self.state
            .tabs
            .selected_page()
            .as_ref()
            .and_then(terminal_of)
    }

    fn title_of(&self, terminal: &vte4::Terminal) -> String {
        terminal
            .window_title()
            .filter(|title| !title.is_empty())
            .map(|title| title.to_string())
            .unwrap_or_else(|| gettext("Terminal"))
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let add = |name: &str, accels: &[&str], run: Rc<dyn Fn(&Self)>| {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| run(&this));
            window.add_action(&action);
            if let Some(app) = window.application() {
                app.set_accels_for_action(&format!("win.{name}"), accels);
            }
        };
        add(
            "new-tab",
            &["<Control><Shift>t"],
            Rc::new(|this| {
                let directory = this
                    .current()
                    .and_then(|terminal| terminal.current_directory_uri())
                    .and_then(|uri| directory_of(&uri));
                this.open_tab(directory);
            }),
        );
        add(
            "close-tab",
            &["<Control><Shift>w"],
            Rc::new(|this| {
                if let Some(page) = this.state.tabs.selected_page() {
                    this.state.tabs.close_page(&page);
                }
            }),
        );
        add(
            "copy",
            &["<Control><Shift>c"],
            Rc::new(|this| {
                if let Some(terminal) = this.current() {
                    terminal.copy_clipboard_format(vte4::Format::Text);
                }
            }),
        );
        add(
            "paste",
            &["<Control><Shift>v"],
            Rc::new(|this| {
                if let Some(terminal) = this.current() {
                    terminal.paste_clipboard();
                }
            }),
        );
        add(
            "select-all",
            &["<Control><Shift>a"],
            Rc::new(|this| {
                if let Some(terminal) = this.current() {
                    terminal.select_all();
                }
            }),
        );
        add(
            "zoom-in",
            &["<Control>plus", "<Control>equal"],
            Rc::new(|this| this.zoom(this.state.zoom.get() + ZOOM_STEP)),
        );
        add(
            "zoom-out",
            &["<Control>minus"],
            Rc::new(|this| this.zoom(this.state.zoom.get() - ZOOM_STEP)),
        );
        add(
            "zoom-reset",
            &["<Control>0"],
            Rc::new(|this| this.zoom(1.0)),
        );
        add(
            "find",
            &["<Control><Shift>f"],
            Rc::new(|this| {
                this.state.search_bar.set_search_mode(true);
                this.state.search.grab_focus();
            }),
        );
        add(
            "find-next",
            &["<Control><Shift>g"],
            Rc::new(|this| this.find(true)),
        );
        add(
            "find-previous",
            &["<Control><Shift>h"],
            Rc::new(|this| this.find(false)),
        );
    }

    fn zoom(&self, scale: f64) {
        let scale = scale.clamp(0.5, 3.0);
        self.state.zoom.set(scale);
        for index in 0..self.state.tabs.n_pages() {
            if let Some(terminal) = terminal_of(&self.state.tabs.nth_page(index)) {
                terminal.set_font_scale(scale);
            }
        }
    }

    fn set_search(&self, text: &str) {
        let Some(terminal) = self.current() else {
            return;
        };
        if text.is_empty() {
            terminal.search_set_regex(None, 0);
            return;
        }
        let pattern = glib::Regex::escape_string(text);
        match vte4::Regex::for_search(&pattern, SEARCH_FLAGS) {
            Ok(regex) => {
                terminal.search_set_regex(Some(&regex), 0);
                terminal.search_set_wrap_around(true);
                terminal.search_find_previous();
            }
            Err(error) => tracing::warn!(%error, "Invalid search pattern"),
        }
    }

    fn find(&self, forward: bool) {
        if let Some(terminal) = self.current() {
            if forward {
                terminal.search_find_next();
            } else {
                terminal.search_find_previous();
            }
        }
    }

    fn open_tab(&self, directory: Option<String>) {
        let terminal = vte4::Terminal::new();
        terminal.set_hexpand(true);
        terminal.set_vexpand(true);
        terminal.set_scrollback_lines(10_000);
        terminal.set_scroll_on_keystroke(true);
        terminal.set_mouse_autohide(true);
        terminal.set_font_scale(self.state.zoom.get());
        // The view background comes from the theme; VTE draws only the text.
        terminal.set_clear_background(false);
        terminal.add_css_class("view");
        if let Ok(settings) = asena_core::settings::load("org.asenalinux.desktop") {
            let font = settings.string("monospace-font-name");
            terminal.set_font(Some(&gtk::pango::FontDescription::from_string(&font)));
        }
        let sync_colors = |terminal: &vte4::Terminal| {
            let foreground = terminal.color();
            terminal.set_colors(Some(&foreground), None, &[]);
        };
        sync_colors(&terminal);
        let weak = terminal.downgrade();
        adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(terminal) = weak.upgrade() {
                sync_colors(&terminal);
            }
        });
        let scroll = gtk::ScrolledWindow::builder()
            .child(&terminal)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let page = self.state.tabs.append(&scroll);
        page.set_title(&gettext("Terminal"));
        let this = self.clone();
        let weak_page = page.downgrade();
        terminal.connect_window_title_changed(move |terminal| {
            let title = this.title_of(terminal);
            if let Some(page) = weak_page.upgrade() {
                page.set_title(&title);
                if this.state.tabs.selected_page().as_ref() == Some(&page) {
                    this.state.window.set_title(Some(&title));
                }
            }
        });
        let tabs = self.state.tabs.clone();
        let weak_page = page.downgrade();
        terminal.connect_child_exited(move |_, _| {
            if let Some(page) = weak_page.upgrade() {
                tabs.close_page(&page);
            }
        });
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        let home = glib::home_dir().to_string_lossy().into_owned();
        let directory = directory.unwrap_or(home);
        terminal.spawn_async(
            vte4::PtyFlags::DEFAULT,
            Some(&directory),
            &[&shell],
            &[],
            glib::SpawnFlags::DEFAULT,
            || {},
            -1,
            gio::Cancellable::NONE,
            move |result| {
                if let Err(error) = result {
                    tracing::warn!(%error, "Could not start the shell");
                }
            },
        );
        self.state.tabs.set_selected_page(&page);
        terminal.grab_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_come_from_file_uris() {
        assert_eq!(
            directory_of("file:///home/asena/Belgeler").as_deref(),
            Some("/home/asena/Belgeler")
        );
        assert_eq!(
            directory_of("file:///tmp/a%20b").as_deref(),
            Some("/tmp/a b")
        );
    }
}
