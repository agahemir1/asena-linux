//! Documents window: a continuous page view of one PDF with zoom and search.
use crate::{
    page::Page,
    poppler::{self, Info},
};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

const SPACING: i32 = 12;
const MARGIN: i32 = 18;
/// GTK lays out in 1/96 inch; PDF sizes are in points, 1/72 inch.
const POINT: f64 = 96.0 / 72.0;

struct Document {
    path: PathBuf,
    info: Info,
    pages: Vec<Page>,
    /// Pages rendered at the current zoom.
    rendered: RefCell<BTreeSet<usize>>,
    text: RefCell<Option<Vec<String>>>,
}

struct State {
    window: AsenaWindow,
    stack: gtk::Stack,
    error: adw::StatusPage,
    title: adw::WindowTitle,
    scroll: gtk::ScrolledWindow,
    column: gtk::Box,
    zoom_label: gtk::Button,
    search_bar: gtk::SearchBar,
    search: gtk::SearchEntry,
    document: RefCell<Option<Rc<Document>>>,
    /// 0 means fit to width.
    zoom: Cell<f64>,
    rendering: Cell<bool>,
    pending: RefCell<Option<glib::SourceId>>,
    generation: Cell<u64>,
}

#[derive(Clone)]
pub struct DocumentsWindow {
    state: Rc<State>,
}

impl DocumentsWindow {
    pub fn new(app: &adw::Application) -> Self {
        let empty = adw::StatusPage::builder()
            .icon_name("x-office-document-symbolic")
            .title(gettext("No document"))
            .build();
        let open = gtk::Button::with_label(&gettext("Open"));
        open.add_css_class("pill");
        open.add_css_class("suggested-action");
        open.set_halign(gtk::Align::Center);
        open.set_action_name(Some("win.open"));
        empty.set_child(Some(&open));
        let error = adw::StatusPage::builder()
            .icon_name("dialog-error-symbolic")
            .title(gettext("The document could not be opened"))
            .build();
        let loading = adw::Spinner::new();
        loading.set_size_request(48, 48);
        loading.set_halign(gtk::Align::Center);
        loading.set_valign(gtk::Align::Center);

        let column = gtk::Box::new(gtk::Orientation::Vertical, SPACING);
        column.set_halign(gtk::Align::Center);
        column.set_margin_top(MARGIN);
        column.set_margin_bottom(MARGIN);
        column.set_margin_start(MARGIN);
        column.set_margin_end(MARGIN);
        let scroll = gtk::ScrolledWindow::builder().child(&column).build();
        scroll.add_css_class("undershoot-top");

        let stack = gtk::Stack::new();
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&error, Some("error"));
        stack.add_named(&scroll, Some("document"));

        let title = adw::WindowTitle::new(&gettext("Documents"), "");
        let open_button = gtk::Button::from_icon_name("document-open-symbolic");
        open_button.set_tooltip_text(Some(&gettext("Open")));
        open_button.set_action_name(Some("win.open"));
        let zoom_out = gtk::Button::from_icon_name("zoom-out-symbolic");
        zoom_out.set_tooltip_text(Some(&gettext("Zoom out")));
        zoom_out.set_action_name(Some("win.zoom-out"));
        let zoom_label = gtk::Button::with_label("");
        zoom_label.set_tooltip_text(Some(&gettext("Fit width")));
        zoom_label.set_action_name(Some("win.zoom-fit"));
        zoom_label.add_css_class("numeric");
        let zoom_in = gtk::Button::from_icon_name("zoom-in-symbolic");
        zoom_in.set_tooltip_text(Some(&gettext("Zoom in")));
        zoom_in.set_action_name(Some("win.zoom-in"));
        let zoom = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        zoom.add_css_class("linked");
        zoom.append(&zoom_out);
        zoom.append(&zoom_label);
        zoom.append(&zoom_in);
        let find = gtk::ToggleButton::new();
        let menu = gio::Menu::new();
        menu.append(
            Some(&gettext("Open with another application")),
            Some("win.open-with"),
        );
        menu.append(Some(&gettext("About Documents")), Some("app.about"));
        let header = AsenaHeaderBarBuilder::new()
            .title_widget(title.clone())
            .pack_start(&open_button)
            .pack_end(&zoom)
            .search_toggle(&find)
            .primary_menu(menu.upcast_ref())
            .build();
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search in document")));
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(&search));
        search_bar.connect_entry(&search);
        find.bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&search_bar);
        toolbar.set_content(Some(&stack));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Documents"))
            .default_size(900, 700)
            .min_size(360, 400)
            .content(&toolbar)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            520.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&zoom_out, "visible", Some(&false.to_value()));
        breakpoint.add_setter(&zoom_in, "visible", Some(&false.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            window,
            stack,
            error,
            title,
            scroll,
            column,
            zoom_label,
            search_bar,
            search,
            document: RefCell::default(),
            zoom: Cell::new(0.0),
            rendering: Cell::new(false),
            pending: RefCell::default(),
            generation: Cell::new(0),
        });
        let this = Self { state };
        this.install_actions();
        let cloned = this.clone();
        this.state
            .scroll
            .vadjustment()
            .connect_value_changed(move |_| cloned.on_scroll());
        let cloned = this.clone();
        this.state
            .scroll
            .vadjustment()
            .connect_page_size_notify(move |_| cloned.relayout());
        let cloned = this.clone();
        this.state
            .scroll
            .hadjustment()
            .connect_page_size_notify(move |_| {
                if cloned.state.zoom.get() == 0.0 {
                    cloned.relayout();
                }
            });
        let cloned = this.clone();
        this.state
            .search
            .connect_activate(move |entry| cloned.find(&entry.text()));
        this.sync_zoom_label();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let add = |name: &str, accels: &[&str], run: fn(&Self)| {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| run(&this));
            window.add_action(&action);
            if let Some(app) = window.application() {
                app.set_accels_for_action(&format!("win.{name}"), accels);
            }
        };
        add("open", &["<Control>o"], |this| this.choose());
        add("open-with", &[], |this| {
            if let Some(document) = this.state.document.borrow().as_ref() {
                let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&document.path)));
                launcher.set_always_ask(true);
                launcher.launch(
                    Some(this.state.window.window()),
                    gio::Cancellable::NONE,
                    |_| {},
                );
            }
        });
        add("zoom-in", &["<Control>plus", "<Control>equal"], |this| {
            this.set_zoom(this.effective_zoom() * 1.2)
        });
        add("zoom-out", &["<Control>minus"], |this| {
            this.set_zoom(this.effective_zoom() / 1.2)
        });
        add("zoom-fit", &["<Control>0"], |this| this.set_zoom(0.0));
        add("find", &["<Control>f"], |this| {
            this.state.search_bar.set_search_mode(true);
            this.state.search.grab_focus();
        });
    }

    fn choose(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(&gettext("PDF documents")));
        filter.add_mime_type("application/pdf");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Open document"))
            .filters(&filters)
            .default_filter(&filter)
            .build();
        let this = self.clone();
        dialog.open(
            Some(self.state.window.window()),
            gio::Cancellable::NONE,
            move |result| {
                if let Some(path) = result.ok().and_then(|file| file.path()) {
                    this.open(path);
                }
            },
        );
    }

    pub fn open(&self, path: PathBuf) {
        let generation = self.state.generation.get() + 1;
        self.state.generation.set(generation);
        self.state.stack.set_visible_child_name("loading");
        let this = self.clone();
        glib::spawn_future_local(async move {
            let lookup = path.clone();
            let result = gio::spawn_blocking(move || poppler::info(&lookup))
                .await
                .unwrap_or_else(|_| Err(poppler::Error::Unreadable("stopped".into())));
            if this.state.generation.get() != generation {
                return;
            }
            match result {
                Ok(info) => this.show(path, info),
                Err(error) => {
                    tracing::warn!(?error, "Could not open the document");
                    let message = match error {
                        poppler::Error::Missing => gettext("PDF support is not installed."),
                        poppler::Error::Encrypted => {
                            gettext("This document is protected with a password.")
                        }
                        poppler::Error::Unreadable(_) => {
                            gettext("The file is damaged or is not a PDF document.")
                        }
                    };
                    this.state.error.set_description(Some(&message));
                    this.state.stack.set_visible_child_name("error");
                }
            }
        });
    }

    fn show(&self, path: PathBuf, info: Info) {
        let state = &self.state;
        while let Some(child) = state.column.first_child() {
            state.column.remove(&child);
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let title = info.title.clone().unwrap_or_else(|| name.clone());
        state.title.set_title(&title);
        state.window.set_title(Some(&title));
        let pages: Vec<Page> = (0..info.sizes.len())
            .map(|index| {
                let picture = Page::default();
                picture.set_halign(gtk::Align::Center);
                picture.add_css_class("card");
                picture.set_overflow(gtk::Overflow::Hidden);
                picture.update_property(&[gtk::accessible::Property::Label(
                    &gettext("Page {number}").replace("{number}", &(index + 1).to_string()),
                )]);
                state.column.append(&picture);
                picture
            })
            .collect();
        state.document.replace(Some(Rc::new(Document {
            path,
            info,
            pages,
            rendered: RefCell::default(),
            text: RefCell::default(),
        })));
        state.stack.set_visible_child_name("document");
        state.scroll.vadjustment().set_value(0.0);
        self.relayout();
    }

    /// Logical pixels per point at the chosen zoom; fit width uses the widest page.
    fn effective_zoom(&self) -> f64 {
        let zoom = self.state.zoom.get();
        if zoom > 0.0 {
            return zoom;
        }
        let Some(document) = self.state.document.borrow().clone() else {
            return 1.0;
        };
        let widest = document
            .info
            .sizes
            .iter()
            .map(|(width, _)| *width)
            .fold(1.0, f64::max);
        let available = (self.state.scroll.width() - 2 * MARGIN - 16).max(120) as f64;
        (available / (widest * POINT)).clamp(0.1, 8.0)
    }

    fn set_zoom(&self, zoom: f64) {
        self.state.zoom.set(if zoom > 0.0 {
            zoom.clamp(0.25, 6.0)
        } else {
            0.0
        });
        self.relayout();
    }

    fn sync_zoom_label(&self) {
        let text = if self.state.zoom.get() > 0.0 {
            format!("{}%", (self.effective_zoom() * 100.0).round() as i64)
        } else {
            gettext("Fit")
        };
        self.state.zoom_label.set_label(&text);
    }

    /// Sizes every page for the zoom and renders the visible ones again.
    fn relayout(&self) {
        self.sync_zoom_label();
        let Some(document) = self.state.document.borrow().clone() else {
            return;
        };
        let zoom = self.effective_zoom();
        for (picture, (width, height)) in document.pages.iter().zip(&document.info.sizes) {
            picture.set_size(
                (width * POINT * zoom).round() as i32,
                (height * POINT * zoom).round() as i32,
            );
        }
        document.rendered.borrow_mut().clear();
        // Wait for the new sizes before choosing visible pages.
        if let Some(source) = self.state.pending.borrow_mut().take() {
            source.remove();
        }
        let this = self.clone();
        let source = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            this.state.pending.borrow_mut().take();
            this.render_visible();
        });
        self.state.pending.replace(Some(source));
    }

    fn on_scroll(&self) {
        self.update_subtitle();
        self.render_visible();
    }

    /// Pages that overlap the viewport, plus one on each side.
    fn visible(&self, document: &Document) -> Vec<usize> {
        let adjustment = self.state.scroll.vadjustment();
        let top = adjustment.value();
        let bottom = top + adjustment.page_size();
        let zoom = self.effective_zoom();
        let mut offset = MARGIN as f64;
        let mut visible = Vec::new();
        for (index, (_, height)) in document.info.sizes.iter().enumerate() {
            let size = height * POINT * zoom;
            if offset + size >= top && offset <= bottom {
                visible.push(index);
            }
            offset += size + SPACING as f64;
        }
        if let (Some(first), Some(last)) = (visible.first().copied(), visible.last().copied()) {
            if first > 0 {
                visible.insert(0, first - 1);
            }
            if last + 1 < document.pages.len() {
                visible.push(last + 1);
            }
        }
        visible
    }

    /// The page that fills the upper third of the view.
    fn current_page(&self, document: &Document) -> usize {
        let adjustment = self.state.scroll.vadjustment();
        let line = adjustment.value() + adjustment.page_size() / 3.0;
        let zoom = self.effective_zoom();
        let mut offset = MARGIN as f64;
        for (index, (_, height)) in document.info.sizes.iter().enumerate() {
            offset += height * POINT * zoom + SPACING as f64;
            if offset > line {
                return index;
            }
        }
        document.pages.len().saturating_sub(1)
    }

    fn update_subtitle(&self) {
        let Some(document) = self.state.document.borrow().clone() else {
            return;
        };
        let current = self.current_page(&document) + 1;
        self.state.title.set_subtitle(
            &gettext("Page {page} of {pages}")
                .replace("{page}", &current.min(document.pages.len()).to_string())
                .replace("{pages}", &document.pages.len().to_string()),
        );
    }

    /// Renders one missing visible page at a time with `pdftoppm`.
    fn render_visible(&self) {
        if self.state.rendering.get() {
            return;
        }
        let Some(document) = self.state.document.borrow().clone() else {
            return;
        };
        self.update_subtitle();
        let Some(page) = self
            .visible(&document)
            .into_iter()
            .find(|page| !document.rendered.borrow().contains(page))
        else {
            return;
        };
        let scale = self.state.window.scale_factor().max(1) as f64;
        let dpi = (72.0 * POINT * self.effective_zoom() * scale)
            .round()
            .clamp(18.0, 1200.0) as u32;
        self.state.rendering.set(true);
        let this = self.clone();
        let path = document.path.clone();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || {
                let png =
                    poppler::render(&path, page + 1, dpi).map_err(|error| format!("{error:?}"))?;
                gtk::gdk::Texture::from_bytes(&glib::Bytes::from_owned(png))
                    .map_err(|error| error.to_string())
            })
            .await
            .unwrap_or_else(|_| Err("stopped".into()));
            this.state.rendering.set(false);
            let current = this.state.document.borrow().clone();
            let Some(current) = current.filter(|current| Rc::ptr_eq(current, &document)) else {
                return;
            };
            match result {
                Ok(texture) => {
                    if let Some(picture) = current.pages.get(page) {
                        picture.set_texture(Some(texture));
                    }
                }
                Err(error) => tracing::warn!(%error, page, "Could not render a page"),
            }
            // Marked even after an error so one bad page does not stop the others.
            current.rendered.borrow_mut().insert(page);
            this.render_visible();
        });
    }

    fn find(&self, query: &str) {
        let Some(document) = self.state.document.borrow().clone() else {
            return;
        };
        let this = self.clone();
        let query = query.to_owned();
        glib::spawn_future_local(async move {
            if document.text.borrow().is_none() {
                let path = document.path.clone();
                let text = gio::spawn_blocking(move || poppler::text(&path))
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or_default();
                document.text.replace(Some(text));
            }
            let matches = document
                .text
                .borrow()
                .as_deref()
                .map(|pages| poppler::find(pages, &query))
                .unwrap_or_default();
            if matches.is_empty() {
                this.state.window.show_toast(&gettext("No matches"));
                return;
            }
            // The next matching page after the one at the top of the view.
            let current = this.current_page(&document);
            let target = matches
                .iter()
                .copied()
                .find(|page| *page > current)
                .unwrap_or(matches[0]);
            this.scroll_to(&document, target);
            this.state.window.show_toast(
                &ngettext(
                    "Found on {count} page",
                    "Found on {count} pages",
                    matches.len() as u32,
                )
                .replace("{count}", &matches.len().to_string()),
            );
        });
    }

    fn scroll_to(&self, document: &Document, page: usize) {
        let zoom = self.effective_zoom();
        let offset: f64 = document.info.sizes[..page.min(document.info.sizes.len())]
            .iter()
            .map(|(_, height)| height * POINT * zoom + SPACING as f64)
            .sum();
        self.state.scroll.vadjustment().set_value(offset);
    }
}
