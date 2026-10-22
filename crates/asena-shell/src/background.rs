use gtk::{gdk, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Pixels {
    bytes: Vec<u8>,
    width: i32,
    height: i32,
    stride: usize,
}

fn load(uri: &str, width: i32, height: i32, mode: &str) -> Result<Option<Pixels>, String> {
    if uri.is_empty() || mode == "solid" {
        return Ok(None);
    }
    let path = gio::File::for_uri(uri)
        .path()
        .ok_or_else(|| "Only local background images are supported".to_string())?;
    let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
    if metadata.len() > 64 * 1024 * 1024
        || width <= 0
        || height <= 0
        || i64::from(width) * i64::from(height) > 33_177_600
    {
        return Err("Background image exceeds resource limits".into());
    }
    use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
    let image = Pixbuf::from_file(&path).map_err(|error| error.to_string())?;
    let factor = match mode {
        "fit" => (f64::from(width) / f64::from(image.width()))
            .min(f64::from(height) / f64::from(image.height())),
        "center" => 1.0,
        _ => (f64::from(width) / f64::from(image.width()))
            .max(f64::from(height) / f64::from(image.height())),
    };
    let scaled_w = (f64::from(image.width()) * factor).round().max(1.0) as i32;
    let scaled_h = (f64::from(image.height()) * factor).round().max(1.0) as i32;
    let canvas = Pixbuf::new(Colorspace::Rgb, true, 8, width, height)
        .ok_or_else(|| "Could not allocate background image".to_string())?;
    canvas.fill(0); // Transparent padding reveals the semantic surface color.
    let x = (width - scaled_w) / 2;
    let y = (height - scaled_h) / 2;
    image.composite(
        &canvas,
        x.max(0),
        y.max(0),
        scaled_w.min(width),
        scaled_h.min(height),
        f64::from(x),
        f64::from(y),
        factor,
        factor,
        InterpType::Bilinear,
        255,
    );
    Ok(Some(Pixels {
        bytes: canvas.read_pixel_bytes().to_vec(),
        width,
        height,
        stride: canvas.rowstride() as usize,
    }))
}

struct State {
    stack: gtk::Stack,
    pictures: [gtk::Picture; 2],
    active: Cell<usize>,
    generation: Cell<u64>,
    task: RefCell<Option<glib::JoinHandle<()>>>,
    /// Pixel size of the image last requested.
    size: Cell<(i32, i32)>,
}

/// Pixels to decode for the monitor: its logical size at the integer scale
/// the compositor renders the background with.
fn pixel_size(monitor: &gdk::Monitor) -> (i32, i32) {
    let (area, scale) = crate::outputs::geometry(monitor);
    let factor = scale.ceil().max(1.0) as i32;
    (area.width() * factor, area.height() * factor)
}

fn update(state: &Rc<State>, monitor: &gdk::Monitor, settings: &gio::Settings) {
    if let Some(task) = state.task.borrow_mut().take() {
        task.abort();
    }
    let generation = state.generation.get().wrapping_add(1);
    state.generation.set(generation);
    let (width, height) = pixel_size(monitor);
    state.size.set((width, height));
    let uri = settings.string("background-uri");
    let mode = settings.string("background-mode");
    let weak = Rc::downgrade(state);
    let task = glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || load(&uri, width, height, &mode)).await;
        let Some(state) = weak.upgrade() else {
            return;
        };
        if state.generation.get() != generation {
            return;
        }
        let next = 1 - state.active.get();
        let pixels = match result {
            Ok(Ok(pixels)) => pixels,
            Ok(Err(error)) => {
                tracing::warn!(%error, "Background image unavailable; using surface color");
                None
            }
            Err(error) => {
                tracing::warn!(?error, "Background worker failed");
                None
            }
        };
        if let Some(pixels) = pixels {
            let texture = gdk::MemoryTexture::new(
                pixels.width,
                pixels.height,
                gdk::MemoryFormat::R8g8b8a8,
                &glib::Bytes::from_owned(pixels.bytes),
                pixels.stride,
            );
            state.pictures[next].set_paintable(Some(&texture));
        } else {
            state.pictures[next].set_paintable(None::<&gdk::Texture>);
        }
        state.stack.set_visible_child(&state.pictures[next]);
        state.active.set(next);
        if !state.stack.is_transition_running() {
            state.pictures[1 - next].set_paintable(None::<&gdk::Texture>);
        }
    });
    state.task.replace(Some(task));
}

pub fn create(
    app: &adw::Application,
    monitor: &gdk::Monitor,
    settings: &gio::Settings,
    icons: bool,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .build();
    window.init_layer_shell();
    window.set_namespace(Some("asena-background"));
    window.set_layer(Layer::Background);
    // Desktop icons take keyboard focus only after the user clicks them.
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_monitor(Some(monitor));
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }
    window.set_exclusive_zone(-1);
    window.add_css_class("asena-background");
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(300);
    let pictures = [gtk::Picture::new(), gtk::Picture::new()];
    for picture in &pictures {
        picture.set_content_fit(gtk::ContentFit::Fill);
        stack.add_child(picture);
    }
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&stack));
    let desktop = crate::desktop::create(&window, settings, icons);
    // The wallpaper runs under the panel; icons and menus stay clear of it.
    let panel = asena_core::settings::load(asena_core::settings::SHELL_SCHEMA)
        .ok()
        .map(|settings| asena_core::layout::PanelLayout::from_settings(&settings))
        .unwrap_or_default();
    match panel.edge {
        asena_core::layout::PanelEdge::Bottom => desktop.set_margin_bottom(panel.thickness),
        asena_core::layout::PanelEdge::Top => desktop.set_margin_top(panel.thickness),
        asena_core::layout::PanelEdge::Left => desktop.set_margin_start(panel.thickness),
        asena_core::layout::PanelEdge::Right => desktop.set_margin_end(panel.thickness),
    }
    overlay.add_overlay(&desktop);
    window.set_child(Some(&overlay));
    let state = Rc::new(State {
        stack,
        pictures,
        active: Cell::new(0),
        generation: Cell::new(0),
        task: RefCell::new(None),
        size: Cell::new((0, 0)),
    });
    let weak = Rc::downgrade(&state);
    state.stack.connect_transition_running_notify(move |stack| {
        if !stack.is_transition_running() {
            if let Some(state) = weak.upgrade() {
                state.pictures[1 - state.active.get()].set_paintable(None::<&gdk::Texture>);
            }
        }
    });
    update(&state, monitor, settings);
    let active_state = state.clone();
    let active_monitor = monitor.clone();
    let setting_handler = settings.connect_changed(None, move |settings, _| {
        update(&active_state, &active_monitor, settings)
    });
    // GTK and the output heads both report monitor changes; decode again only
    // when the size actually changed.
    let resized = {
        let state = state.clone();
        let prefs = settings.clone();
        Rc::new(move |monitor: &gdk::Monitor| {
            if state.size.get() != pixel_size(monitor) {
                update(&state, monitor, &prefs);
            }
        })
    };
    let on_notify = resized.clone();
    let monitor_handler = monitor.connect_notify_local(None, move |monitor, property| {
        if ["geometry", "scale-factor"].contains(&property.name()) {
            on_notify(monitor);
        }
    });
    let heads_monitor = monitor.clone();
    let heads_handler = crate::outputs::connect_changed(move || resized(&heads_monitor));
    let settings = settings.clone();
    let monitor = monitor.clone();
    let handlers = RefCell::new(Some((setting_handler, monitor_handler)));
    window.connect_unrealize(move |_| {
        if let Some((a, b)) = handlers.borrow_mut().take() {
            settings.disconnect(a);
            monitor.disconnect(b);
            crate::outputs::disconnect(heads_handler);
        }
        if let Some(task) = state.task.borrow_mut().take() {
            task.abort();
        }
    });
    window
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn scaling_modes_keep_transparent_padding_and_crop_zoom() {
        let uri = gio::File::for_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/background.svg"
        ))
        .uri();
        for (mode, corner_alpha) in [("zoom", 255), ("fit", 0), ("center", 0)] {
            let pixels = load(&uri, 8, 8, mode).unwrap().unwrap();
            assert_eq!((pixels.width, pixels.height), (8, 8));
            assert_eq!(pixels.bytes[3], corner_alpha);
            assert_eq!(pixels.bytes[3 * pixels.stride + 3 * 4 + 3], 255);
        }
        assert!(load(&uri, 0, 8, "fit").is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn solid_and_missing_images_are_recoverable() {
        assert!(load("", 8, 8, "zoom").unwrap().is_none());
        assert!(load("file:///missing-asena-image.png", 8, 8, "solid")
            .unwrap()
            .is_none());
        assert!(load("file:///missing-asena-image.png", 8, 8, "zoom").is_err());
        assert!(load("https://example.invalid/image.png", 8, 8, "zoom").is_err());
    }
}
