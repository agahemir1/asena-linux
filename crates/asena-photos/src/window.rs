//! Photos window: a thumbnail grid of the Pictures folder and an image viewer.
use crate::photo::{scan, siblings, step, Photo};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

const THUMBNAIL: i32 = 320;
const FULL: i32 = 3840;

struct Pixels {
    bytes: Vec<u8>,
    width: i32,
    height: i32,
    stride: usize,
    alpha: bool,
}

/// Decodes on a worker thread, honoring the camera orientation tag.
fn decode(path: &Path, size: i32) -> Option<Pixels> {
    if std::fs::metadata(path).ok()?.len() > 256 * 1024 * 1024 {
        return None;
    }
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(path, size, size, true).ok()?;
    let pixbuf = pixbuf.apply_embedded_orientation().unwrap_or(pixbuf);
    Some(Pixels {
        bytes: pixbuf.read_pixel_bytes().to_vec(),
        width: pixbuf.width(),
        height: pixbuf.height(),
        stride: pixbuf.rowstride() as usize,
        alpha: pixbuf.has_alpha(),
    })
}

fn texture(pixels: Pixels) -> gtk::gdk::Texture {
    gtk::gdk::MemoryTexture::new(
        pixels.width,
        pixels.height,
        if pixels.alpha {
            gtk::gdk::MemoryFormat::R8g8b8a8
        } else {
            gtk::gdk::MemoryFormat::R8g8b8
        },
        &glib::Bytes::from_owned(pixels.bytes),
        pixels.stride,
    )
    .upcast()
}

struct State {
    window: AsenaWindow,
    navigation: adw::NavigationView,
    library_title: adw::WindowTitle,
    stack: gtk::Stack,
    paths: gtk::StringList,
    photos: RefCell<Vec<Photo>>,
    cache: RefCell<HashMap<String, gtk::gdk::Texture>>,
    viewer: adw::NavigationPage,
    viewer_title: adw::WindowTitle,
    picture: gtk::Picture,
    browsing: RefCell<Vec<Photo>>,
    index: Cell<usize>,
    generation: Cell<u64>,
}

#[derive(Clone)]
pub struct PhotosWindow {
    state: Rc<State>,
}

fn pictures_dir() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Pictures)
        .unwrap_or_else(|| glib::home_dir().join("Pictures"))
}

impl PhotosWindow {
    pub fn new(app: &adw::Application) -> Self {
        let paths = gtk::StringList::new(&[]);
        let selection = gtk::NoSelection::new(Some(paths.clone()));
        let factory = gtk::SignalListItemFactory::new();
        let grid = gtk::GridView::new(Some(selection), Some(factory.clone()));
        grid.set_max_columns(12);
        grid.set_min_columns(2);
        grid.set_single_click_activate(true);
        grid.add_css_class("asena-photo-grid");

        let library_title = adw::WindowTitle::new(&gettext("Photos"), "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&library_title));
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("Choose folder…")), Some("win.choose-folder"));
        menu.append(Some(&gettext("About Photos")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);
        let empty = adw::StatusPage::builder()
            .icon_name("image-x-generic-symbolic")
            .title(gettext("No photos"))
            .build();
        let choose = gtk::Button::with_label(&gettext("Choose folder…"));
        choose.add_css_class("pill");
        choose.set_halign(gtk::Align::Center);
        choose.set_action_name(Some("win.choose-folder"));
        empty.set_child(Some(&choose));
        let loading = adw::Spinner::new();
        loading.set_halign(gtk::Align::Center);
        loading.set_valign(gtk::Align::Center);
        loading.set_size_request(48, 48);
        let stack = gtk::Stack::new();
        stack.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&grid)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .build(),
            Some("grid"),
        );
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&loading, Some("loading"));
        let library_view = adw::ToolbarView::new();
        library_view.add_top_bar(&header);
        library_view.set_content(Some(&stack));
        let library = adw::NavigationPage::builder()
            .title(gettext("Photos"))
            .tag("library")
            .child(&library_view)
            .build();

        let picture = gtk::Picture::new();
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_vexpand(true);
        picture.add_css_class("asena-photo-view");
        let viewer_title = adw::WindowTitle::new("", "");
        let viewer_header = adw::HeaderBar::new();
        viewer_header.set_title_widget(Some(&viewer_title));
        let previous = gtk::Button::from_icon_name("go-previous-symbolic");
        previous.set_tooltip_text(Some(&gettext("Previous photo")));
        previous.set_action_name(Some("win.previous"));
        let next = gtk::Button::from_icon_name("go-next-symbolic");
        next.set_tooltip_text(Some(&gettext("Next photo")));
        next.set_action_name(Some("win.next"));
        let viewer_menu = gio::Menu::new();
        viewer_menu.append(Some(&gettext("Set as background")), Some("win.background"));
        viewer_menu.append(Some(&gettext("Show in Files")), Some("win.show-folder"));
        viewer_menu.append(Some(&gettext("Move to trash")), Some("win.trash"));
        let viewer_menu_button = gtk::MenuButton::new();
        viewer_menu_button.set_icon_name("view-more-symbolic");
        viewer_menu_button.set_tooltip_text(Some(&gettext("More")));
        viewer_menu_button.set_menu_model(Some(&viewer_menu));
        viewer_header.pack_end(&viewer_menu_button);
        viewer_header.pack_end(&next);
        viewer_header.pack_end(&previous);
        let viewer_view = adw::ToolbarView::new();
        viewer_view.add_top_bar(&viewer_header);
        viewer_view.set_content(Some(&picture));
        let viewer = adw::NavigationPage::builder()
            .title(gettext("Photo"))
            .tag("viewer")
            .child(&viewer_view)
            .build();

        let navigation = adw::NavigationView::new();
        navigation.add(&library);
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Photos"))
            .icon_name("org.asenalinux.Photos")
            .default_size(960, 640)
            .min_size(360, 360)
            .content(&navigation)
            .build();

        let state = Rc::new(State {
            window,
            navigation,
            library_title,
            stack,
            paths,
            photos: RefCell::default(),
            cache: RefCell::default(),
            viewer,
            viewer_title,
            picture,
            browsing: RefCell::default(),
            index: Cell::new(0),
            generation: Cell::new(0),
        });
        let this = Self { state };
        this.setup_factory(&factory);
        let cloned = this.clone();
        grid.connect_activate(move |_, position| {
            let photos = cloned.state.photos.borrow().clone();
            cloned.show(photos, position as usize);
        });
        this.setup_actions();
        this.load_folder(pictures_dir());
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    /// Views an image passed on the command line, browsing its folder.
    pub fn open(&self, file: &gio::File) {
        let Some(path) = file.path() else {
            return;
        };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let target = path.clone();
            let photos = gio::spawn_blocking(move || siblings(&target))
                .await
                .unwrap_or_default();
            let index = photos
                .iter()
                .position(|photo| photo.path == path)
                .unwrap_or(0);
            if !photos.is_empty() {
                this.show(photos, index);
            }
        });
    }

    fn setup_factory(&self, factory: &gtk::SignalListItemFactory) {
        factory.connect_setup(|_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let picture = gtk::Picture::new();
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture.set_size_request(150, 150);
            picture.add_css_class("asena-photo-thumbnail");
            let frame = gtk::Frame::new(None);
            frame.set_child(Some(&picture));
            frame.set_overflow(gtk::Overflow::Hidden);
            item.set_child(Some(&frame));
        });
        let this = self.clone();
        factory.connect_bind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let (Some(path), Some(frame)) = (
                item.item().and_downcast::<gtk::StringObject>(),
                item.child().and_downcast::<gtk::Frame>(),
            ) else {
                return;
            };
            let Some(picture) = frame.child().and_downcast::<gtk::Picture>() else {
                return;
            };
            let path = path.string().to_string();
            picture.set_widget_name(&path);
            if let Some(texture) = this.state.cache.borrow().get(&path) {
                picture.set_paintable(Some(texture));
                return;
            }
            picture.set_paintable(None::<&gtk::gdk::Paintable>);
            let weak = picture.downgrade();
            let state = Rc::downgrade(&this.state);
            glib::spawn_future_local(async move {
                let source = PathBuf::from(&path);
                let pixels = gio::spawn_blocking(move || decode(&source, THUMBNAIL))
                    .await
                    .ok()
                    .flatten();
                let (Some(pixels), Some(state)) = (pixels, state.upgrade()) else {
                    return;
                };
                let texture = texture(pixels);
                state
                    .cache
                    .borrow_mut()
                    .insert(path.clone(), texture.clone());
                // The row may have been recycled for another photo meanwhile.
                if let Some(picture) = weak.upgrade() {
                    if picture.widget_name() == path {
                        picture.set_paintable(Some(&texture));
                    }
                }
            });
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
        add("previous", &["Left"], Box::new(|this| this.browse(false)));
        add("next", &["Right"], Box::new(|this| this.browse(true)));
        add(
            "choose-folder",
            &["<Control>o"],
            Box::new(|this| {
                let dialog = gtk::FileDialog::new();
                let this = this.clone();
                let parent = this.state.window.window().clone();
                dialog.select_folder(Some(&parent), gio::Cancellable::NONE, move |result| {
                    if let Some(path) = result.ok().and_then(|folder| folder.path()) {
                        this.load_folder(path);
                    }
                });
            }),
        );
        add(
            "background",
            &[],
            Box::new(|this| {
                let Some(photo) = this.current() else {
                    return;
                };
                match asena_core::settings::load("org.asenalinux.desktop") {
                    Ok(settings) => {
                        let uri = gio::File::for_path(&photo.path).uri();
                        if settings.set_string("background-uri", &uri).is_ok() {
                            this.state.window.show_toast(&gettext("Background changed"));
                        }
                    }
                    Err(error) => show_error_dialog(
                        this.state.window.window(),
                        &gettext("Could not change the background"),
                        &error.to_string(),
                    ),
                }
            }),
        );
        add(
            "show-folder",
            &[],
            Box::new(|this| {
                if let Some(photo) = this.current() {
                    let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&photo.path)));
                    launcher.open_containing_folder(
                        Some(this.state.window.window()),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                }
            }),
        );
        add("trash", &["Delete"], Box::new(|this| this.trash()));
        let actions_enabled = |state: &State, enabled: bool| {
            for name in ["previous", "next", "background", "show-folder", "trash"] {
                if let Some(action) = state
                    .window
                    .lookup_action(name)
                    .and_downcast::<gio::SimpleAction>()
                {
                    action.set_enabled(enabled);
                }
            }
        };
        actions_enabled(&self.state, false);
        let weak = Rc::downgrade(&self.state);
        self.state.viewer.connect_shown(move |_| {
            if let Some(state) = weak.upgrade() {
                actions_enabled(&state, true);
            }
        });
        let weak = Rc::downgrade(&self.state);
        self.state.viewer.connect_hidden(move |_| {
            if let Some(state) = weak.upgrade() {
                actions_enabled(&state, false);
                state.picture.set_paintable(None::<&gtk::gdk::Paintable>);
            }
        });
    }

    fn load_folder(&self, folder: PathBuf) {
        self.state.stack.set_visible_child_name("loading");
        self.state
            .library_title
            .set_subtitle(&gio::File::for_path(&folder).parse_name());
        let this = self.clone();
        glib::spawn_future_local(async move {
            let photos = gio::spawn_blocking(move || scan(&folder))
                .await
                .unwrap_or_default();
            let paths: Vec<String> = photos
                .iter()
                .map(|photo| photo.path.to_string_lossy().into_owned())
                .collect();
            let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
            let state = &this.state;
            state.paths.splice(0, state.paths.n_items(), &refs);
            state
                .stack
                .set_visible_child_name(if photos.is_empty() { "empty" } else { "grid" });
            state.photos.replace(photos);
        });
    }

    fn current(&self) -> Option<Photo> {
        self.state
            .browsing
            .borrow()
            .get(self.state.index.get())
            .cloned()
    }

    fn show(&self, photos: Vec<Photo>, index: usize) {
        self.state.browsing.replace(photos);
        self.state.index.set(index);
        if self.state.navigation.visible_page().as_ref() != Some(&self.state.viewer) {
            self.state.navigation.push(&self.state.viewer);
        }
        self.load_current();
    }

    fn browse(&self, forward: bool) {
        let len = self.state.browsing.borrow().len();
        self.state
            .index
            .set(step(self.state.index.get(), len, forward));
        self.load_current();
    }

    fn load_current(&self) {
        let Some(photo) = self.current() else {
            return;
        };
        let generation = self.state.generation.get().wrapping_add(1);
        self.state.generation.set(generation);
        let name = photo
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let count = self.state.browsing.borrow().len();
        self.state.viewer_title.set_title(&name);
        self.state.viewer_title.set_subtitle(
            &gettext("{index} of {count}")
                .replace("{index}", &(self.state.index.get() + 1).to_string())
                .replace("{count}", &count.to_string()),
        );
        // Show the cached thumbnail at once, then the full image.
        let key = photo.path.to_string_lossy().into_owned();
        let cached = self.state.cache.borrow().get(&key).cloned();
        self.state.picture.set_paintable(cached.as_ref());
        let this = self.clone();
        glib::spawn_future_local(async move {
            let path = photo.path.clone();
            let pixels = gio::spawn_blocking(move || decode(&path, FULL))
                .await
                .ok()
                .flatten();
            if this.state.generation.get() != generation {
                return;
            }
            match pixels {
                Some(pixels) => this.state.picture.set_paintable(Some(&texture(pixels))),
                None => this
                    .state
                    .window
                    .show_toast(&gettext("Could not open the image")),
            }
        });
    }

    fn trash(&self) {
        let Some(photo) = self.current() else {
            return;
        };
        let this = self.clone();
        gio::File::for_path(&photo.path).trash_async(
            glib::Priority::DEFAULT,
            gio::Cancellable::NONE,
            move |result| match result {
                Ok(()) => {
                    let key = photo.path.to_string_lossy().into_owned();
                    this.state
                        .photos
                        .borrow_mut()
                        .retain(|item| item.path != photo.path);
                    this.state
                        .browsing
                        .borrow_mut()
                        .retain(|item| item.path != photo.path);
                    let paths = &this.state.paths;
                    if let Some(position) = (0..paths.n_items())
                        .find(|index| paths.string(*index).as_deref() == Some(key.as_str()))
                    {
                        paths.remove(position);
                    }
                    this.state
                        .window
                        .show_toast(&gettext("Photo moved to the trash"));
                    let len = this.state.browsing.borrow().len();
                    if len == 0 {
                        this.state.navigation.pop();
                    } else {
                        this.state.index.set(this.state.index.get().min(len - 1));
                        this.load_current();
                    }
                }
                Err(error) => show_error_dialog(
                    this.state.window.window(),
                    &gettext("Could not move the photo to the trash"),
                    error.message(),
                ),
            },
        );
    }
}
