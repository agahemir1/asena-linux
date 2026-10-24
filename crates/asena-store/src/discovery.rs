//! Catalog-backed discovery: a browsable showcase and bounded application rails.
use crate::{
    catalog::{self, App, Installed},
    screenshots,
    window::image,
};
use asena_app::prelude::*;
use asena_core::launcher::Category;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type OpenApp = Rc<dyn Fn(&App)>;
type OpenCategory = Rc<dyn Fn(Category)>;

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.add_css_class(class);
    label
}

fn arrow(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.add_css_class("flat");
    button.add_css_class("circular");
    button.set_tooltip_text(Some(tooltip));
    button
}

fn horizontal(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(child)
        .build()
}

fn on_screen(widget: &gtk::Widget) -> bool {
    if !widget.is_mapped() {
        return false;
    }
    let mut ancestor = widget.parent();
    while let Some(parent) = ancestor {
        if parent.is::<gtk::ScrolledWindow>() || parent.is::<adw::Carousel>() {
            let Some(bounds) = widget.compute_bounds(&parent) else {
                return false;
            };
            if bounds.x() + bounds.width() <= 0.0
                || bounds.y() + bounds.height() <= 0.0
                || bounds.x() >= parent.width() as f32
                || bounds.y() >= parent.height() as f32
            {
                return false;
            }
        }
        ancestor = parent.parent();
    }
    true
}

/// Requests start when a card enters the viewport. Scroll signals, rather than
/// a permanent frame callback, wake waiting cards. Detached cards own no tasks.
fn load_preview(stack: &gtk::Overlay, urls: Vec<String>, height: i32) {
    let started = Rc::new(Cell::new(false));
    let weak = stack.downgrade();
    let check: Rc<dyn Fn()> = Rc::new(move || {
        let Some(stack) = weak.upgrade() else {
            return;
        };
        if started.get() || !on_screen(stack.upcast_ref()) {
            return;
        }
        started.set(true);
        let weak = stack.downgrade();
        let urls = urls.clone();
        glib::spawn_future_local(async move {
            for url in urls.into_iter().take(3) {
                if weak.upgrade().is_none() {
                    return;
                }
                let Some(texture) = screenshots::load(url).await else {
                    continue;
                };
                let Some(stack) = weak.upgrade() else {
                    return;
                };
                let picture = gtk::Picture::for_paintable(&texture);
                picture.set_can_shrink(true);
                picture.set_content_fit(if height > 144 {
                    gtk::ContentFit::Cover
                } else {
                    gtk::ContentFit::Contain
                });
                picture.set_hexpand(true);
                picture.set_vexpand(true);
                stack.add_overlay(&picture);
                stack.set_measure_overlay(&picture, false);
                stack.set_clip_overlay(&picture, true);
                if let Some(icon) = stack.child() {
                    icon.set_opacity(0.0);
                }
                return;
            }
        });
    });
    type Subscription = (glib::Object, glib::SignalHandlerId);
    let subscriptions = Rc::new(RefCell::new(Vec::<Subscription>::new()));
    let mapped = subscriptions.clone();
    stack.connect_map(move |stack| {
        let mut ancestor = stack.parent();
        while let Some(parent) = ancestor {
            if let Some(scroll) = parent.downcast_ref::<gtk::ScrolledWindow>() {
                for adjustment in [scroll.hadjustment(), scroll.vadjustment()] {
                    let wake = check.clone();
                    let handler = adjustment.connect_value_changed(move |_| wake());
                    mapped
                        .borrow_mut()
                        .push((adjustment.clone().upcast(), handler));
                    let wake = check.clone();
                    let handler = adjustment.connect_changed(move |_| wake());
                    mapped.borrow_mut().push((adjustment.upcast(), handler));
                }
            } else if let Some(carousel) = parent.downcast_ref::<adw::Carousel>() {
                let wake = check.clone();
                let handler = carousel.connect_position_notify(move |_| wake());
                mapped
                    .borrow_mut()
                    .push((carousel.clone().upcast(), handler));
            }
            ancestor = parent.parent();
        }
        let wake = check.clone();
        // Mapping can precede the first allocation. Check once layout exists,
        // then stop ticking; offscreen cards wait for viewport signals.
        stack.add_tick_callback(move |widget, _| {
            if widget.width() <= 0 || widget.height() <= 0 {
                return glib::ControlFlow::Continue;
            }
            wake();
            glib::ControlFlow::Break
        });
    });
    stack.connect_unmap(move |_| {
        for (object, handler) in subscriptions.borrow_mut().drain(..) {
            object.disconnect(handler);
        }
    });
}

/// A failed request leaves the real icon in place and the card remains usable.
fn preview(app: &App, height: i32) -> gtk::Overlay {
    let stack = gtk::Overlay::new();
    stack.add_css_class("store-preview");
    stack.set_height_request(height);
    stack.set_hexpand(true);
    stack.set_overflow(gtk::Overflow::Hidden);
    let placeholder = image(app.icon.as_ref(), 80);
    stack.set_child(Some(&placeholder));
    if !app.screenshots.is_empty() {
        load_preview(&stack, app.screenshots.clone(), height);
    }
    stack
}

fn metadata(app: &App, installed: &Installed, large: bool) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("store-metadata");
    row.append(&image(app.icon.as_ref(), if large { 56 } else { 48 }));
    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let title = label(&app.name, if large { "title-2" } else { "heading" });
    title.set_max_width_chars(if large { 40 } else { 22 });
    text.append(&title);
    let summary = label(&app.summary, "dim-label");
    summary.set_wrap(true);
    summary.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    summary.set_lines(2);
    summary.set_max_width_chars(if large { 48 } else { 22 });
    text.append(&summary);
    if installed.any(app) {
        let state = label(&gettext("Installed"), "caption");
        state.add_css_class("accent");
        text.append(&state);
    }
    row.append(&text);
    row
}

fn card(app: &App, installed: &Installed, screenshot: bool, open: &OpenApp) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("store-card");
    button.set_tooltip_text(Some(&app.name));
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    if screenshot {
        content.append(&preview(app, 144));
    }
    content.append(&metadata(app, installed, false));
    button.set_child(Some(&content));
    button.set_size_request(if screenshot { 264 } else { 240 }, -1);
    let app = app.clone();
    let open = open.clone();
    button.connect_clicked(move |_| open(&app));
    button
}

fn showcase(apps: &[&App], installed: &Installed, open: &OpenApp) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = label(&gettext("In the spotlight"), "title-1");
    title.set_hexpand(true);
    header.append(&title);
    let previous = arrow("go-previous-symbolic", &gettext("Previous application"));
    let next = arrow("go-next-symbolic", &gettext("Next application"));
    header.append(&previous);
    header.append(&next);
    section.append(&header);
    let carousel = adw::Carousel::new();
    carousel.set_spacing(16);
    carousel.set_allow_long_swipes(true);
    for app in apps {
        let button = gtk::Button::new();
        button.add_css_class("store-card");
        button.add_css_class("store-feature");
        button.set_hexpand(true);
        button.set_tooltip_text(Some(&app.name));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&preview(app, 224));
        let footer = metadata(app, installed, true);
        footer.append(&gtk::Image::from_icon_name("go-next-symbolic"));
        content.append(&footer);
        button.set_child(Some(&content));
        let app = (*app).clone();
        let open = open.clone();
        button.connect_clicked(move |_| open(&app));
        carousel.append(&button);
    }
    for (button, direction) in [(&previous, -1.0), (&next, 1.0)] {
        let weak = carousel.downgrade();
        button.connect_clicked(move |_| {
            let Some(carousel) = weak.upgrade() else {
                return;
            };
            let index = (carousel.position().round() + direction)
                .clamp(0.0, f64::from(carousel.n_pages().saturating_sub(1)))
                as u32;
            carousel.scroll_to(&carousel.nth_page(index), true);
        });
    }
    let previous_state = previous.clone();
    let next_state = next.clone();
    carousel.connect_position_notify(move |carousel| {
        previous_state.set_sensitive(carousel.position() > 0.01);
        next_state.set_sensitive(
            carousel.position() < f64::from(carousel.n_pages().saturating_sub(1)) - 0.01,
        );
    });
    previous.set_sensitive(false);
    next.set_sensitive(apps.len() > 1);
    header.set_visible(!apps.is_empty());
    previous.set_visible(apps.len() > 1);
    next.set_visible(apps.len() > 1);
    section.append(&carousel);
    if apps.len() > 1 {
        let dots = adw::CarouselIndicatorDots::new();
        dots.set_carousel(Some(&carousel));
        section.append(&dots);
    }
    section
}

fn rail(
    category: Category,
    apps: &[&App],
    installed: &Installed,
    open: &OpenApp,
    browse: &OpenCategory,
) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = label(&category.name(), "title-2");
    title.set_hexpand(true);
    header.append(&title);
    let all = gtk::Button::with_label(&gettext("See all"));
    all.add_css_class("flat");
    let browse = browse.clone();
    all.connect_clicked(move |_| browse(category));
    header.append(&all);
    let previous = arrow("go-previous-symbolic", &gettext("Scroll back"));
    let next = arrow("go-next-symbolic", &gettext("Scroll forward"));
    header.append(&previous);
    header.append(&next);
    section.append(&header);
    let cards = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    // Screenshots add useful visual content in creative and game categories.
    let visual = matches!(
        category,
        Category::Graphics | Category::Games | Category::Multimedia
    );
    for app in apps {
        cards.append(&card(app, installed, visual, open));
    }
    let scroll = horizontal(&cards);
    let adjustment = scroll.hadjustment();
    for (button, direction) in [(&previous, -1.0), (&next, 1.0)] {
        let adjustment = adjustment.clone();
        button.connect_clicked(move |_| {
            let step = adjustment.page_size() * 0.85;
            adjustment.set_value((adjustment.value() + step * direction).clamp(
                adjustment.lower(),
                (adjustment.upper() - adjustment.page_size()).max(adjustment.lower()),
            ));
        });
    }
    let update = Rc::new(move |adjustment: &gtk::Adjustment| {
        previous.set_sensitive(adjustment.value() > adjustment.lower() + 1.0);
        next.set_sensitive(adjustment.value() + adjustment.page_size() < adjustment.upper() - 1.0);
    });
    let changed = update.clone();
    adjustment.connect_changed(move |adjustment| changed(adjustment));
    adjustment.connect_value_changed(move |adjustment| update(adjustment));
    section.append(&scroll);
    section
}

pub fn fill(
    root: &gtk::Box,
    apps: &[App],
    installed: &Installed,
    open: OpenApp,
    browse: OpenCategory,
) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
    if apps.is_empty() {
        let empty = adw::StatusPage::builder()
            .icon_name("system-software-install-symbolic")
            .title(gettext("The catalog is empty"))
            .description(gettext(
                "Refresh to download the application lists, or enable Flathub in the main menu.",
            ))
            .build();
        root.append(&empty);
        return;
    }
    let categories = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for category in Category::ALL {
        if !apps.iter().any(|app| app.category() == category) {
            continue;
        }
        let button = gtk::Button::new();
        button.add_css_class("pill");
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        content.append(&gtk::Image::from_icon_name(category.icon()));
        content.append(&gtk::Label::new(Some(&category.name())));
        button.set_child(Some(&content));
        let browse = browse.clone();
        button.connect_clicked(move |_| browse(category));
        categories.append(&button);
    }
    root.append(&horizontal(&categories));
    let featured = catalog::featured(apps, 4);
    if !featured.is_empty() {
        root.append(&showcase(&featured, installed, &open));
    }
    for category in [
        Category::Graphics,
        Category::Office,
        Category::Multimedia,
        Category::Games,
        Category::Internet,
        Category::Development,
        Category::Education,
        Category::Utilities,
        Category::System,
        Category::Other,
    ] {
        let selection = catalog::discover(apps, Some(category), 8);
        if !selection.is_empty() {
            root.append(&rail(category, &selection, installed, &open, &browse));
        }
    }
}

pub fn view(root: &gtk::Box) -> gtk::ScrolledWindow {
    root.set_spacing(28);
    root.set_margin_top(20);
    root.set_margin_bottom(32);
    root.set_margin_start(20);
    root.set_margin_end(20);
    let clamp = adw::Clamp::builder()
        .maximum_size(1120)
        .tightening_threshold(900)
        .child(root)
        .build();
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&clamp)
        .build()
}

pub fn install_style() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("discovery.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
