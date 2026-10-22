//! Media player window: library of music and videos, and the playback page.
use crate::library::{matches, scan, track, Kind, Track};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

struct State {
    window: AsenaWindow,
    navigation: adw::NavigationView,
    library_stack: gtk::Stack,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    tracks: RefCell<Vec<Track>>,
    player: adw::NavigationPage,
    title: adw::WindowTitle,
    stage: gtk::Stack,
    video: gtk::Video,
    audio_title: gtk::Label,
    controls: gtk::MediaControls,
    stream: RefCell<Option<gtk::MediaFile>>,
    queue: RefCell<Vec<Track>>,
    index: Cell<usize>,
}

#[derive(Clone)]
pub struct PlayerWindow {
    state: Rc<State>,
}

fn library_roots() -> Vec<PathBuf> {
    [glib::UserDirectory::Music, glib::UserDirectory::Videos]
        .into_iter()
        .filter_map(glib::user_special_dir)
        .filter(|path| path != &glib::home_dir())
        .collect()
}

impl PlayerWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Library -----------------------------------------------------------------
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search music and videos")));
        search.set_hexpand(true);
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(
            &adw::Clamp::builder()
                .maximum_size(480)
                .child(&search)
                .build(),
        ));
        search_bar.connect_entry(&search);
        let header = adw::HeaderBar::new();
        let open = gtk::Button::with_label(&gettext("Open"));
        open.set_action_name(Some("win.open"));
        header.pack_start(&open);
        let search_toggle = gtk::ToggleButton::new();
        search_toggle.set_icon_name("edit-find-symbolic");
        search_toggle.set_tooltip_text(Some(&gettext("Search")));
        search_toggle
            .bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("Reload library")), Some("win.reload"));
        menu.append(Some(&gettext("About Media Player")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        header.pack_end(&search_toggle);
        let empty = adw::StatusPage::builder()
            .icon_name("audio-x-generic-symbolic")
            .title(gettext("No music or videos"))
            .build();
        let open_empty = gtk::Button::with_label(&gettext("Open files…"));
        open_empty.add_css_class("pill");
        open_empty.set_halign(gtk::Align::Center);
        open_empty.set_action_name(Some("win.open"));
        empty.set_child(Some(&open_empty));
        let library_stack = gtk::Stack::new();
        library_stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("list"),
        );
        library_stack.add_named(&empty, Some("empty"));
        let library_view = adw::ToolbarView::new();
        library_view.add_top_bar(&header);
        library_view.add_top_bar(&search_bar);
        library_view.set_content(Some(&library_stack));
        let library = adw::NavigationPage::builder()
            .title(gettext("Media Player"))
            .tag("library")
            .child(&library_view)
            .build();

        // Playback ------------------------------------------------------------------
        let video = gtk::Video::new();
        video.set_autoplay(true);
        video.set_vexpand(true);
        let audio_icon = gtk::Image::from_icon_name("audio-x-generic-symbolic");
        audio_icon.set_pixel_size(96);
        audio_icon.add_css_class("dim-label");
        let audio_title = gtk::Label::new(None);
        audio_title.add_css_class("title-2");
        audio_title.set_wrap(true);
        audio_title.set_justify(gtk::Justification::Center);
        let audio = gtk::Box::new(gtk::Orientation::Vertical, 18);
        audio.set_valign(gtk::Align::Center);
        audio.set_vexpand(true);
        audio.append(&audio_icon);
        audio.append(&audio_title);
        let stage = gtk::Stack::new();
        stage.add_named(&video, Some("video"));
        stage.add_named(&audio, Some("audio"));
        let controls = gtk::MediaControls::new(None::<&gtk::MediaStream>);
        controls.set_margin_start(12);
        controls.set_margin_end(12);
        controls.set_margin_bottom(12);
        let title = adw::WindowTitle::new("", "");
        let player_header = adw::HeaderBar::new();
        player_header.set_title_widget(Some(&title));
        let previous = gtk::Button::from_icon_name("media-skip-backward-symbolic");
        previous.set_tooltip_text(Some(&gettext("Previous")));
        previous.set_action_name(Some("win.previous"));
        let next = gtk::Button::from_icon_name("media-skip-forward-symbolic");
        next.set_tooltip_text(Some(&gettext("Next")));
        next.set_action_name(Some("win.next"));
        player_header.pack_end(&next);
        player_header.pack_end(&previous);
        let player_view = adw::ToolbarView::new();
        player_view.add_top_bar(&player_header);
        player_view.set_content(Some(&stage));
        player_view.add_bottom_bar(&controls);
        let player = adw::NavigationPage::builder()
            .title(gettext("Now playing"))
            .tag("player")
            .child(&player_view)
            .build();

        let navigation = adw::NavigationView::new();
        navigation.add(&library);
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Media Player"))
            .default_size(860, 560)
            .min_size(360, 360)
            .content(&navigation)
            .build();

        let state = Rc::new(State {
            window,
            navigation,
            library_stack,
            list,
            search,
            tracks: RefCell::default(),
            player,
            title,
            stage,
            video,
            audio_title,
            controls,
            stream: RefCell::default(),
            queue: RefCell::default(),
            index: Cell::new(0),
        });
        let this = Self { state };
        this.setup();
        this.reload();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    pub fn play_files(&self, files: &[gio::File]) {
        let queue: Vec<Track> = files
            .iter()
            .filter_map(|file| file.path())
            .filter_map(track)
            .collect();
        if queue.is_empty() {
            self.state
                .window
                .show_toast(&gettext("These files cannot be played"));
            return;
        }
        self.play(queue, 0);
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
            "open",
            &["<Control>o"],
            Box::new(|this| {
                let dialog = gtk::FileDialog::new();
                let parent = this.state.window.window().clone();
                let this = this.clone();
                dialog.open_multiple(Some(&parent), gio::Cancellable::NONE, move |result| {
                    if let Ok(model) = result {
                        let files: Vec<gio::File> = (0..model.n_items())
                            .filter_map(|index| model.item(index).and_downcast::<gio::File>())
                            .collect();
                        this.play_files(&files);
                    }
                });
            }),
        );
        add("reload", &["F5"], Box::new(|this| this.reload()));
        add(
            "previous",
            &["<Control>Left"],
            Box::new(|this| this.skip(false)),
        );
        add(
            "next",
            &["<Control>Right"],
            Box::new(|this| this.skip(true)),
        );
        add(
            "toggle",
            &["<Control>space"],
            Box::new(|this| {
                if let Some(stream) = this.state.stream.borrow().as_ref() {
                    stream.set_playing(!stream.is_playing());
                }
            }),
        );
        let this = self.clone();
        self.state
            .search
            .connect_search_changed(move |_| this.render());
        let this = self.clone();
        self.state.list.connect_row_activated(move |_, row| {
            let query = this.state.search.text();
            let visible: Vec<Track> = this
                .state
                .tracks
                .borrow()
                .iter()
                .filter(|track| matches(track, &query))
                .cloned()
                .collect();
            let index = row.index().max(0) as usize;
            this.play(visible, index);
        });
        let weak = Rc::downgrade(&self.state);
        self.state.player.connect_hidden(move |_| {
            if let Some(state) = weak.upgrade() {
                if let Some(stream) = state.stream.borrow_mut().take() {
                    stream.set_playing(false);
                }
                state.video.set_media_stream(None::<&gtk::MediaStream>);
                state.controls.set_media_stream(None::<&gtk::MediaStream>);
            }
        });
    }

    fn reload(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let roots = library_roots();
            let mut tracks = gio::spawn_blocking(move || scan(&roots))
                .await
                .unwrap_or_default();
            tracks.sort_by(|a, b| {
                glib::CollationKey::from(a.title.as_str())
                    .cmp(&glib::CollationKey::from(b.title.as_str()))
            });
            this.state.tracks.replace(tracks);
            this.render();
        });
    }

    fn render(&self) {
        let list = &self.state.list;
        list.remove_all();
        let query = self.state.search.text();
        let tracks = self.state.tracks.borrow();
        for track in tracks.iter().filter(|track| matches(track, &query)) {
            let row = adw::ActionRow::new();
            row.set_title(&glib::markup_escape_text(&track.title));
            if let Some(folder) = track.path.parent().and_then(|path| path.file_name()) {
                row.set_subtitle(&glib::markup_escape_text(&folder.to_string_lossy()));
            }
            row.add_prefix(&gtk::Image::from_icon_name(match track.kind {
                Kind::Audio => "audio-x-generic-symbolic",
                Kind::Video => "video-x-generic-symbolic",
            }));
            row.set_activatable(true);
            list.append(&row);
        }
        self.state
            .library_stack
            .set_visible_child_name(if tracks.is_empty() { "empty" } else { "list" });
    }

    fn play(&self, queue: Vec<Track>, index: usize) {
        if queue.is_empty() {
            return;
        }
        self.state.queue.replace(queue);
        self.state.index.set(index);
        if self.state.navigation.visible_page().as_ref() != Some(&self.state.player) {
            self.state.navigation.push(&self.state.player);
        }
        self.load_current();
    }

    fn skip(&self, forward: bool) {
        let len = self.state.queue.borrow().len();
        let index = self.state.index.get();
        let next = if forward {
            (index + 1 < len).then_some(index + 1)
        } else {
            index.checked_sub(1)
        };
        if let Some(next) = next {
            self.state.index.set(next);
            self.load_current();
        }
    }

    fn load_current(&self) {
        let Some(track) = self
            .state
            .queue
            .borrow()
            .get(self.state.index.get())
            .cloned()
        else {
            return;
        };
        if let Some(previous) = self.state.stream.borrow_mut().take() {
            previous.set_playing(false);
        }
        let stream = gtk::MediaFile::for_filename(&track.path);
        let len = self.state.queue.borrow().len();
        self.state.title.set_title(&track.title);
        self.state.title.set_subtitle(&if len > 1 {
            gettext("{index} of {count}")
                .replace("{index}", &(self.state.index.get() + 1).to_string())
                .replace("{count}", &len.to_string())
        } else {
            String::new()
        });
        self.state.audio_title.set_text(&track.title);
        match track.kind {
            Kind::Video => {
                self.state.stage.set_visible_child_name("video");
                self.state.video.set_media_stream(Some(&stream));
            }
            Kind::Audio => {
                self.state.stage.set_visible_child_name("audio");
                self.state.video.set_media_stream(None::<&gtk::MediaStream>);
            }
        }
        self.state.controls.set_media_stream(Some(&stream));
        let this = self.clone();
        stream.connect_ended_notify(move |stream| {
            if stream.is_ended() {
                this.skip(true);
            }
        });
        let window = self.state.window.clone();
        stream.connect_error_notify(move |stream| {
            if let Some(error) = stream.error() {
                window.show_toast(
                    &gettext("Playback failed: {error}").replace("{error}", error.message()),
                );
            }
        });
        stream.play();
        for (name, enabled) in [
            ("previous", self.state.index.get() > 0),
            ("next", self.state.index.get() + 1 < len),
        ] {
            if let Some(action) = self
                .state
                .window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                action.set_enabled(enabled);
            }
        }
        self.state.stream.replace(Some(stream));
    }
}
