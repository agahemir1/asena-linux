//! Camera window: live viewfinder, photos and video recording through GStreamer.
use crate::capture::{self, Kind};
use adw::prelude::*;
use asena_app::prelude::*;
use gst::prelude::DeviceExt as GstDeviceExt;
use gst::prelude::*;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

/// The running viewfinder for one camera.
struct Viewfinder {
    pipeline: gst::Pipeline,
    tee: gst::Element,
    sink: gst::Element,
    _bus: gst::bus::BusWatchGuard,
}

impl Drop for Viewfinder {
    fn drop(&mut self) {
        // Releases the device for other applications.
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

struct Recording {
    bin: gst::Bin,
    pad: gst::Pad,
    path: PathBuf,
    started: Instant,
    tick: glib::SourceId,
}

struct State {
    window: AsenaWindow,
    stack: gtk::Stack,
    error: adw::StatusPage,
    picture: gtk::Picture,
    cameras: gtk::DropDown,
    names: gtk::StringList,
    devices: RefCell<Vec<gst::Device>>,
    microphone: Cell<bool>,
    monitor: gst::DeviceMonitor,
    monitor_bus: RefCell<Option<gst::bus::BusWatchGuard>>,
    viewfinder: RefCell<Option<Viewfinder>>,
    mode: Cell<Kind>,
    shutter: gtk::Button,
    photo: gtk::ToggleButton,
    elapsed: gtk::Label,
    last: gtk::Button,
    last_file: RefCell<Option<PathBuf>>,
    recording: RefCell<Option<Recording>>,
    busy: Cell<bool>,
}

#[derive(Clone)]
pub struct CameraWindow {
    state: Rc<State>,
}

fn element(name: &str) -> Result<gst::Element, glib::BoolError> {
    gst::ElementFactory::make(name).build()
}

fn stamp() -> String {
    glib::DateTime::now_local()
        .and_then(|now| now.format("%Y-%m-%d %H-%M-%S"))
        .map(|text| text.to_string())
        .unwrap_or_default()
}

impl CameraWindow {
    pub fn new(app: &adw::Application) -> Self {
        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_hexpand(true);
        picture.set_vexpand(true);

        let photo = gtk::ToggleButton::builder()
            .icon_name("camera-photo-symbolic")
            .tooltip_text(gettext("Photo"))
            .active(true)
            .build();
        let video = gtk::ToggleButton::builder()
            .icon_name("camera-video-symbolic")
            .tooltip_text(gettext("Video"))
            .group(&photo)
            .build();
        let modes = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        modes.add_css_class("linked");
        modes.append(&photo);
        modes.append(&video);
        modes.set_valign(gtk::Align::Center);

        let shutter = gtk::Button::from_icon_name("camera-photo-symbolic");
        shutter.add_css_class("circular");
        shutter.add_css_class("suggested-action");
        shutter.set_size_request(64, 64);
        shutter.set_tooltip_text(Some(&gettext("Take photo")));
        shutter.set_action_name(Some("win.shutter"));

        let last = gtk::Button::new();
        last.add_css_class("flat");
        last.set_size_request(56, 56);
        last.set_valign(gtk::Align::Center);
        last.set_tooltip_text(Some(&gettext("Open last capture")));
        last.set_visible(false);

        let elapsed = gtk::Label::new(None);
        elapsed.add_css_class("numeric");
        elapsed.add_css_class("title-4");
        elapsed.set_visible(false);

        let controls = gtk::CenterBox::new();
        controls.set_margin_top(12);
        controls.set_margin_bottom(18);
        controls.set_margin_start(18);
        controls.set_margin_end(18);
        controls.set_start_widget(Some(&modes));
        let center = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        center.append(&shutter);
        center.append(&elapsed);
        controls.set_center_widget(Some(&center));
        controls.set_end_widget(Some(&last));

        let view = gtk::Box::new(gtk::Orientation::Vertical, 0);
        view.append(&picture);
        view.append(&controls);

        let none = adw::StatusPage::builder()
            .icon_name("camera-disabled-symbolic")
            .title(gettext("No camera found"))
            .build();
        let error = adw::StatusPage::builder()
            .icon_name("camera-disabled-symbolic")
            .title(gettext("The camera could not be started"))
            .build();
        let loading = adw::Spinner::new();
        loading.set_size_request(48, 48);
        loading.set_halign(gtk::Align::Center);
        loading.set_valign(gtk::Align::Center);
        let stack = gtk::Stack::new();
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&none, Some("none"));
        stack.add_named(&error, Some("error"));
        stack.add_named(&view, Some("view"));
        stack.set_visible_child_name("loading");

        let names = gtk::StringList::new(&[]);
        let cameras = gtk::DropDown::new(Some(names.clone()), None::<gtk::Expression>);
        cameras.set_tooltip_text(Some(&gettext("Camera")));
        cameras.set_visible(false);
        let menu = gio::Menu::new();
        menu.append(
            Some(&gettext("Open photos folder")),
            Some("win.open-photos"),
        );
        menu.append(
            Some(&gettext("Open videos folder")),
            Some("win.open-videos"),
        );
        menu.append(Some(&gettext("About Camera")), Some("app.about"));
        let header = AsenaHeaderBarBuilder::new()
            .pack_start(&cameras)
            .primary_menu(menu.upcast_ref())
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Camera"))
            .default_size(750, 550)
            .min_size(360, 350)
            .content(&toolbar)
            .build();

        let monitor = gst::DeviceMonitor::new();
        monitor.add_filter(Some("Video/Source"), None);
        monitor.add_filter(Some("Audio/Source"), None);
        let state = Rc::new(State {
            window,
            stack,
            error,
            picture,
            cameras,
            names,
            devices: RefCell::default(),
            microphone: Cell::new(false),
            monitor,
            monitor_bus: RefCell::default(),
            viewfinder: RefCell::default(),
            mode: Cell::new(Kind::Photo),
            shutter,
            photo,
            elapsed,
            last,
            last_file: RefCell::default(),
            recording: RefCell::default(),
            busy: Cell::new(false),
        });
        let this = Self { state };
        this.install_actions();
        let cloned = this.clone();
        this.state.photo.connect_toggled(move |button| {
            if cloned.state.recording.borrow().is_some() {
                return;
            }
            let kind = if button.is_active() {
                Kind::Photo
            } else {
                Kind::Video
            };
            cloned.state.mode.set(kind);
            cloned.sync_shutter();
        });
        let cloned = this.clone();
        this.state
            .cameras
            .connect_selected_notify(move |_| cloned.open_selected());
        let cloned = this.clone();
        this.state.last.connect_clicked(move |_| {
            if let Some(path) = cloned.state.last_file.borrow().clone() {
                gtk::FileLauncher::new(Some(&gio::File::for_path(path))).launch(
                    Some(cloned.state.window.window()),
                    gio::Cancellable::NONE,
                    |_| {},
                );
            }
        });
        let cloned = this.clone();
        this.state.window.connect_close_request(move |_| {
            cloned.stop_recording();
            cloned.state.viewfinder.replace(None);
            cloned.state.monitor.stop();
            glib::Propagation::Proceed
        });
        this.start_monitor();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let shutter = gio::SimpleAction::new("shutter", None);
        let this = self.clone();
        shutter.connect_activate(move |_, _| match this.state.mode.get() {
            Kind::Photo => this.take_photo(),
            Kind::Video => {
                if this.state.recording.borrow().is_some() {
                    this.stop_recording();
                } else {
                    this.start_recording();
                }
            }
        });
        window.add_action(&shutter);
        if let Some(app) = window.application() {
            app.set_accels_for_action("win.shutter", &["space", "Return"]);
        }
        for (name, kind) in [("open-photos", Kind::Photo), ("open-videos", Kind::Video)] {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| {
                let this = this.clone();
                glib::spawn_future_local(async move {
                    let directory = gio::spawn_blocking(move || {
                        let directory = capture::directory(kind);
                        std::fs::create_dir_all(&directory).map(|_| directory).ok()
                    })
                    .await
                    .ok()
                    .flatten();
                    if let Some(directory) = directory {
                        gtk::FileLauncher::new(Some(&gio::File::for_path(directory))).launch(
                            Some(this.state.window.window()),
                            gio::Cancellable::NONE,
                            |_| {},
                        );
                    }
                });
            });
            window.add_action(&action);
        }
    }

    fn sync_shutter(&self) {
        let state = &self.state;
        let recording = state.recording.borrow().is_some();
        let (icon, tooltip) = match (state.mode.get(), recording) {
            (Kind::Photo, _) => ("camera-photo-symbolic", gettext("Take photo")),
            (Kind::Video, false) => ("media-record-symbolic", gettext("Start recording")),
            (Kind::Video, true) => ("media-playback-stop-symbolic", gettext("Stop recording")),
        };
        state.shutter.set_icon_name(icon);
        state.shutter.set_tooltip_text(Some(&tooltip));
        if recording {
            state.shutter.remove_css_class("suggested-action");
            state.shutter.add_css_class("destructive-action");
        } else {
            state.shutter.remove_css_class("destructive-action");
            state.shutter.add_css_class("suggested-action");
        }
        state.elapsed.set_visible(recording);
        state.cameras.set_sensitive(!recording);
        state.photo.set_sensitive(!recording);
        state.shutter.set_sensitive(!state.busy.get());
    }

    /// Starting the monitor probes devices, which can take a moment.
    fn start_monitor(&self) {
        let monitor = self.state.monitor.clone();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let started = gio::spawn_blocking(move || monitor.start().is_ok())
                .await
                .unwrap_or(false);
            if !started {
                tracing::warn!("The camera device monitor did not start");
            }
            let cloned = this.clone();
            let guard = this.state.monitor.bus().add_watch_local(move |_, message| {
                if matches!(
                    message.view(),
                    gst::MessageView::DeviceAdded(_) | gst::MessageView::DeviceRemoved(_)
                ) {
                    cloned.refresh_devices();
                }
                glib::ControlFlow::Continue
            });
            this.state.monitor_bus.replace(guard.ok());
            this.refresh_devices();
        });
    }

    fn refresh_devices(&self) {
        let state = &self.state;
        let all: Vec<gst::Device> = state.monitor.devices().into_iter().collect();
        state.microphone.set(
            all.iter()
                .any(|device| device.device_class().contains("Audio/Source")),
        );
        let cameras: Vec<gst::Device> = all
            .into_iter()
            .filter(|device| device.device_class().contains("Video/Source"))
            .collect();
        let current = state
            .devices
            .borrow()
            .get(state.cameras.selected() as usize)
            .map(|device| device.display_name());
        let unchanged = {
            let devices = state.devices.borrow();
            devices.len() == cameras.len()
                && devices
                    .iter()
                    .zip(&cameras)
                    .all(|(a, b)| a.display_name() == b.display_name())
        };
        if unchanged && state.viewfinder.borrow().is_some() {
            return;
        }
        let names: Vec<String> = cameras
            .iter()
            .map(|device| device.display_name().to_string())
            .collect();
        let selected = current
            .and_then(|name| names.iter().position(|other| *other == name.as_str()))
            .unwrap_or(0);
        state.devices.replace(cameras);
        state.names.splice(
            0,
            state.names.n_items(),
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        state.cameras.set_visible(names.len() > 1);
        if names.is_empty() {
            self.stop_recording();
            state.viewfinder.replace(None);
            state.stack.set_visible_child_name("none");
            return;
        }
        if state.cameras.selected() as usize != selected {
            // The notify handler opens the camera.
            state.cameras.set_selected(selected as u32);
        } else {
            self.open_selected();
        }
    }

    fn open_selected(&self) {
        let state = &self.state;
        let Some(device) = state
            .devices
            .borrow()
            .get(state.cameras.selected() as usize)
            .cloned()
        else {
            return;
        };
        self.stop_recording();
        state.viewfinder.replace(None);
        match self.build(&device) {
            Ok(viewfinder) => {
                let paintable = viewfinder.sink.property::<gtk::gdk::Paintable>("paintable");
                state.picture.set_paintable(Some(&paintable));
                if viewfinder.pipeline.set_state(gst::State::Playing).is_err() {
                    self.fail("Could not start the camera pipeline");
                    return;
                }
                state.viewfinder.replace(Some(viewfinder));
                state.stack.set_visible_child_name("view");
                self.sync_shutter();
            }
            Err(error) => self.fail(&error.to_string()),
        }
    }

    fn fail(&self, detail: &str) {
        tracing::warn!(detail, "Camera failed");
        self.stop_recording();
        self.state.viewfinder.replace(None);
        self.state.error.set_description(Some(&gettext(
            "Another application may be using it. Close that application and open Camera again.",
        )));
        self.state.stack.set_visible_child_name("error");
    }

    /// source ! decodebin ! videoconvert ! tee ! queue ! videoconvert ! gtk4paintablesink;
    /// recordings attach to the tee.
    fn build(&self, device: &gst::Device) -> Result<Viewfinder, glib::BoolError> {
        let pipeline = gst::Pipeline::new();
        let source = device.create_element(None)?;
        let decode = element("decodebin")?;
        let convert = element("videoconvert")?;
        let tee = element("tee")?;
        tee.set_property("allow-not-linked", true);
        let queue = element("queue")?;
        let preview = element("videoconvert")?;
        let sink = element("gtk4paintablesink")?;
        pipeline.add_many([&source, &decode, &convert, &tee, &queue, &preview, &sink])?;
        source.link(&decode)?;
        gst::Element::link_many([&convert, &tee])?;
        gst::Element::link_many([&queue, &preview, &sink])?;
        tee.link(&queue)?;
        let target = convert.downgrade();
        decode.connect_pad_added(move |_, pad| {
            let Some(convert) = target.upgrade() else {
                return;
            };
            let is_video = pad
                .current_caps()
                .and_then(|caps| caps.structure(0).map(|s| s.name().starts_with("video/")))
                .unwrap_or(true);
            if let Some(sink) = convert.static_pad("sink").filter(|sink| !sink.is_linked()) {
                if is_video {
                    let _ = pad.link(&sink);
                }
            }
        });
        let bus = pipeline
            .bus()
            .ok_or_else(|| glib::bool_error!("The pipeline has no bus"))?;
        let this = self.clone();
        let guard = bus.add_watch_local(move |_, message| {
            if let gst::MessageView::Error(error) = message.view() {
                this.fail(&error.error().to_string());
            }
            glib::ControlFlow::Continue
        })?;
        Ok(Viewfinder {
            pipeline,
            tee,
            sink,
            _bus: guard,
        })
    }

    fn take_photo(&self) {
        let Some(sample) = self
            .state
            .viewfinder
            .borrow()
            .as_ref()
            .and_then(|viewfinder| {
                viewfinder
                    .sink
                    .property::<Option<gst::Sample>>("last-sample")
            })
        else {
            return;
        };
        self.state.busy.set(true);
        self.sync_shutter();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let name = capture::file_name(Kind::Photo, &stamp());
            let result = gio::spawn_blocking(
                move || -> Result<(PathBuf, Option<gtk::gdk::Texture>), String> {
                    let jpeg = gst_video::convert_sample(
                        &sample,
                        &gst::Caps::builder("image/jpeg").build(),
                        gst::ClockTime::from_seconds(5),
                    )
                    .map_err(|error| error.to_string())?;
                    let buffer = jpeg.buffer().ok_or("The photo is empty")?;
                    let map = buffer.map_readable().map_err(|error| error.to_string())?;
                    let directory = capture::directory(Kind::Photo);
                    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
                    let path = capture::unique(&directory, &name, |path| path.exists());
                    asena_app::storage::write_atomic(&path, map.as_slice())
                        .map_err(|error| error.to_string())?;
                    // Decoded here so the thumbnail does not load on the main thread.
                    let texture =
                        gtk::gdk::Texture::from_bytes(&glib::Bytes::from(map.as_slice())).ok();
                    Ok((path, texture))
                },
            )
            .await
            .unwrap_or_else(|_| Err("The photo task stopped".into()));
            this.state.busy.set(false);
            this.sync_shutter();
            match result {
                Ok((path, texture)) => this.show_last(path, texture),
                Err(error) => {
                    tracing::warn!(%error, "Could not save the photo");
                    this.state
                        .window
                        .show_toast(&gettext("Could not save the photo"));
                }
            }
        });
    }

    fn show_last(&self, path: PathBuf, texture: Option<gtk::gdk::Texture>) {
        let state = &self.state;
        let image = gtk::Picture::new();
        image.set_paintable(texture.as_ref());
        image.set_content_fit(gtk::ContentFit::Cover);
        image.set_size_request(48, 48);
        let frame = gtk::Frame::new(None);
        frame.set_child(Some(&image));
        frame.set_overflow(gtk::Overflow::Hidden);
        state.last.set_child(Some(&frame));
        state.last.set_visible(true);
        state.last_file.replace(Some(path));
    }

    fn start_recording(&self) {
        if self.state.busy.replace(true) {
            return;
        }
        self.sync_shutter();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let name = capture::file_name(Kind::Video, &stamp());
            let path = gio::spawn_blocking(move || {
                let directory = capture::directory(Kind::Video);
                std::fs::create_dir_all(&directory).ok()?;
                Some(capture::unique(&directory, &name, |path| path.exists()))
            })
            .await
            .ok()
            .flatten();
            this.state.busy.set(false);
            match path {
                Some(path) => this.attach_recorder(path),
                None => this
                    .state
                    .window
                    .show_toast(&gettext("Could not start recording")),
            }
            this.sync_shutter();
        });
    }

    fn attach_recorder(&self, path: PathBuf) {
        let state = &self.state;
        let Some((pipeline, tee)) = state
            .viewfinder
            .borrow()
            .as_ref()
            .map(|viewfinder| (viewfinder.pipeline.clone(), viewfinder.tee.clone()))
        else {
            return;
        };
        let result = (|| -> Result<Recording, glib::BoolError> {
            let description = capture::recording_description(&path, state.microphone.get());
            let bin = gst::parse::bin_from_description(&description, true)
                .map_err(|error| glib::bool_error!("{error}"))?;
            pipeline.add(&bin)?;
            let pad = tee
                .request_pad_simple("src_%u")
                .ok_or_else(|| glib::bool_error!("The camera has no free output"))?;
            let sink = bin
                .static_pad("sink")
                .ok_or_else(|| glib::bool_error!("The recorder has no input"))?;
            pad.link(&sink)
                .map_err(|error| glib::bool_error!("{error:?}"))?;
            bin.sync_state_with_parent()?;
            let this = self.clone();
            let tick = glib::timeout_add_local(Duration::from_secs(1), move || {
                if let Some(recording) = this.state.recording.borrow().as_ref() {
                    this.state
                        .elapsed
                        .set_text(&capture::elapsed(recording.started.elapsed().as_secs()));
                }
                glib::ControlFlow::Continue
            });
            Ok(Recording {
                bin,
                pad,
                path: path.clone(),
                started: Instant::now(),
                tick,
            })
        })();
        match result {
            Ok(recording) => {
                state.elapsed.set_text(&capture::elapsed(0));
                state.recording.replace(Some(recording));
                self.sync_shutter();
            }
            Err(error) => {
                tracing::warn!(%error, "Could not start recording");
                state
                    .window
                    .show_toast(&gettext("Could not start recording"));
            }
        }
    }

    /// Detaches the recorder and lets the muxer finish the file before it is removed.
    fn stop_recording(&self) {
        let Some(recording) = self.state.recording.borrow_mut().take() else {
            return;
        };
        recording.tick.remove();
        let (finished, done) = async_channel::bounded::<()>(1);
        if let Some(file_pad) = recording
            .bin
            .by_name("file")
            .and_then(|file| file.static_pad("sink"))
        {
            file_pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, info| {
                if matches!(
                    info.event().map(|event| event.type_()),
                    Some(gst::EventType::Eos)
                ) {
                    let _ = finished.send_blocking(());
                    return gst::PadProbeReturn::Remove;
                }
                gst::PadProbeReturn::Ok
            });
        }
        let bin = recording.bin.clone();
        recording
            .pad
            .add_probe(gst::PadProbeType::BLOCK_DOWNSTREAM, move |pad, _| {
                if let Some(peer) = pad.peer() {
                    let _ = pad.unlink(&peer);
                    peer.send_event(gst::event::Eos::new());
                }
                if let Some(microphone) = bin.by_name("microphone") {
                    microphone.send_event(gst::event::Eos::new());
                }
                gst::PadProbeReturn::Remove
            });
        let pipeline = self
            .state
            .viewfinder
            .borrow()
            .as_ref()
            .map(|viewfinder| (viewfinder.pipeline.clone(), viewfinder.tee.clone()));
        let this = self.clone();
        glib::spawn_future_local(async move {
            // A stalled muxer must not keep the file open for ever.
            let timeout = glib::timeout_future(Duration::from_secs(10));
            futures_lite::future::or(async { done.recv().await.ok() }, async {
                timeout.await;
                None
            })
            .await;
            let _ = recording.bin.set_state(gst::State::Null);
            if let Some((pipeline, tee)) = pipeline {
                let _ = pipeline.remove(&recording.bin);
                tee.release_request_pad(&recording.pad);
            }
            this.show_last_video(recording.path);
        });
        self.sync_shutter();
    }

    fn show_last_video(&self, path: PathBuf) {
        let state = &self.state;
        let image = gtk::Image::from_icon_name("camera-video-symbolic");
        image.set_pixel_size(24);
        state.last.set_child(Some(&image));
        state.last.set_visible(true);
        state.last_file.replace(Some(path));
        state.window.show_toast(&gettext("Video saved"));
    }
}
