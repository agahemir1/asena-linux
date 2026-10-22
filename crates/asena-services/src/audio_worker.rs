//! libpulse + GLib on a dedicated context. No native audio I/O runs on GTK.
use asena_core::i18n::gettext;
use gio::prelude::*;
use libpulse_binding as pulse;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub channels: u8,
    pub level: f64,
    pub muted: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub available: bool,
    pub sinks: BTreeMap<String, Device>,
    pub sources: BTreeMap<String, Device>,
    pub sink: Option<String>,
    pub source: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Change {
    Level(f64),
    Step(f64),
    Mute,
    Microphone,
    DefaultSink(String),
}
#[derive(Clone)]
enum Expected {
    Level(String, f64),
    Mute(String, bool),
    Microphone(String, bool),
    Sink(String),
}
impl Expected {
    fn matches(&self, snapshot: &Snapshot) -> bool {
        match self {
            Self::Level(name, level) => snapshot
                .sinks
                .get(name)
                .is_some_and(|d| (d.level - level).abs() < 0.0001),
            Self::Mute(name, muted) => snapshot.sinks.get(name).is_some_and(|d| d.muted == *muted),
            Self::Microphone(name, muted) => snapshot
                .sources
                .get(name)
                .is_some_and(|d| d.muted == *muted),
            Self::Sink(name) => snapshot.sink.as_ref() == Some(name),
        }
    }
}
struct Response {
    expected: Expected,
    reply: async_channel::Sender<Result<Snapshot, String>>,
}
pub enum Work {
    Connect,
    State,
    Refresh,
    Request(Change, async_channel::Sender<Result<Snapshot, String>>),
    Stop,
}
trait Pending {
    fn running(&self) -> bool;
    fn cancel(&mut self);
}
impl<T: ?Sized> Pending for pulse::operation::Operation<T> {
    fn running(&self) -> bool {
        self.get_state() == pulse::operation::State::Running
    }
    fn cancel(&mut self) {
        pulse::operation::Operation::cancel(self);
    }
}
struct Worker {
    native: libpulse_glib_binding::Mainloop,
    context: RefCell<Option<pulse::context::Context>>,
    commands: async_channel::Sender<Work>,
    events: async_channel::Sender<Snapshot>,
    snapshot: RefCell<Snapshot>,
    staging: RefCell<Snapshot>,
    generation: Cell<u64>,
    epoch: Cell<u64>,
    responses: RefCell<Vec<Response>>,
    pending: RefCell<Vec<Box<dyn Pending>>>,
    server: Option<String>,
}
impl Worker {
    fn track<T: ?Sized + 'static>(&self, operation: pulse::operation::Operation<T>) {
        let mut pending = self.pending.borrow_mut();
        pending.retain(|op| op.running());
        pending.push(Box::new(operation));
    }
    fn cancel(&self) {
        for mut operation in self.pending.borrow_mut().drain(..) {
            if operation.running() {
                operation.cancel();
            }
        }
    }
    fn disconnect(&self) {
        self.epoch.set(self.epoch.get().wrapping_add(1));
        for response in self.responses.borrow_mut().drain(..) {
            let _ = response
                .reply
                .try_send(Err(gettext("The audio service is unavailable.")));
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        self.cancel();
        if let Some(mut context) = self.context.borrow_mut().take() {
            context.set_state_callback(None);
            context.set_subscribe_callback(None);
            context.disconnect();
        }
        self.snapshot.replace(Snapshot::default());
        let _ = self.events.try_send(Snapshot::default());
    }
    fn connect(self: &Rc<Self>) {
        if self.context.borrow().as_ref().is_some_and(|c| {
            matches!(
                c.get_state(),
                pulse::context::State::Ready
                    | pulse::context::State::Connecting
                    | pulse::context::State::Authorizing
                    | pulse::context::State::SettingName
            )
        }) {
            return;
        }
        self.disconnect();
        let Some(mut context) = pulse::context::Context::new(&self.native, "Asena") else {
            return;
        };
        let commands = self.commands.clone();
        context.set_state_callback(Some(Box::new(move || {
            let _ = commands.try_send(Work::State);
        })));
        let result = context.connect(
            self.server.as_deref(),
            pulse::context::FlagSet::NOAUTOSPAWN,
            None,
        );
        if let Err(error) = result {
            tracing::debug!(%error,"Audio server is unavailable");
            return;
        }
        self.context.replace(Some(context));
    }
    fn state(self: &Rc<Self>) {
        let state = self.context.borrow().as_ref().map(|c| c.get_state());
        match state {
            Some(pulse::context::State::Ready) => {
                let commands = self.commands.clone();
                if let Some(context) = self.context.borrow_mut().as_mut() {
                    context.set_subscribe_callback(Some(Box::new(move |_, _, _| {
                        let _ = commands.try_send(Work::Refresh);
                    })));
                    self.track(context.subscribe(
                        pulse::context::subscribe::InterestMaskSet::SERVER
                            | pulse::context::subscribe::InterestMaskSet::SINK
                            | pulse::context::subscribe::InterestMaskSet::SOURCE,
                        |success| {
                            if !success {
                                tracing::warn!("Audio subscription failed");
                            }
                        },
                    ));
                }
                self.refresh(None);
            }
            Some(pulse::context::State::Failed | pulse::context::State::Terminated) => {
                let reconnect = self.snapshot.borrow().available;
                self.disconnect();
                if reconnect {
                    self.connect();
                }
            }
            _ => {}
        }
    }
    fn refresh(self: &Rc<Self>, reply: Option<Response>) {
        let ready = self
            .context
            .borrow()
            .as_ref()
            .is_some_and(|c| c.get_state() == pulse::context::State::Ready);
        if !ready {
            if let Some(response) = reply {
                let _ = response
                    .reply
                    .try_send(Err(gettext("The audio service is unavailable.")));
            }
            return;
        }
        if let Some(reply) = reply {
            self.responses.borrow_mut().push(reply);
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let weak = Rc::downgrade(self);
        let operation = self.context.borrow().as_ref().map(|context| {
            context.introspect().get_server_info(move |info| {
                let Some(worker) = weak.upgrade() else {
                    return;
                };
                if worker.generation.get() != generation {
                    return;
                }
                worker.staging.replace(Snapshot {
                    available: true,
                    sink: info.default_sink_name.as_ref().map(ToString::to_string),
                    source: info.default_source_name.as_ref().map(ToString::to_string),
                    ..Default::default()
                });
                worker.devices(generation, None);
            })
        });
        if let Some(operation) = operation {
            self.track(operation);
        }
    }
    fn devices(
        self: &Rc<Self>,
        generation: u64,
        reply: Option<async_channel::Sender<Result<Snapshot, String>>>,
    ) {
        let remaining = Rc::new(Cell::new(2_u8));
        let weak = Rc::downgrade(self);
        let pending = remaining.clone();
        let response = reply.clone();
        let sinks = self.context.borrow().as_ref().map(|context| {
            context.introspect().get_sink_info_list(move |result| {
                let Some(worker) = weak.upgrade() else {
                    return;
                };
                if worker.generation.get() != generation {
                    return;
                }
                match result {
                    pulse::callbacks::ListResult::Item(info) => {
                        if let Some(name) = info.name.as_ref() {
                            let name = name.to_string();
                            worker.staging.borrow_mut().sinks.insert(
                                name.clone(),
                                Device {
                                    name,
                                    description: info.description.as_deref().unwrap_or("").into(),
                                    channels: info.volume.len(),
                                    level: f64::from(info.volume.avg().0)
                                        / f64::from(pulse::volume::Volume::NORMAL.0),
                                    muted: info.mute,
                                },
                            );
                        }
                    }
                    _ => worker.finish(&pending, response.as_ref()),
                }
            })
        });
        if let Some(operation) = sinks {
            self.track(operation);
        }
        let weak = Rc::downgrade(self);
        let sources = self.context.borrow().as_ref().map(|context| {
            context.introspect().get_source_info_list(move |result| {
                let Some(worker) = weak.upgrade() else {
                    return;
                };
                if worker.generation.get() != generation {
                    return;
                }
                match result {
                    pulse::callbacks::ListResult::Item(info) => {
                        if info.monitor_of_sink.is_none() {
                            if let Some(name) = info.name.as_ref() {
                                let name = name.to_string();
                                worker.staging.borrow_mut().sources.insert(
                                    name.clone(),
                                    Device {
                                        name,
                                        description: info
                                            .description
                                            .as_deref()
                                            .unwrap_or("")
                                            .into(),
                                        channels: info.volume.len(),
                                        level: f64::from(info.volume.avg().0)
                                            / f64::from(pulse::volume::Volume::NORMAL.0),
                                        muted: info.mute,
                                    },
                                );
                            }
                        }
                    }
                    _ => worker.finish(&remaining, reply.as_ref()),
                }
            })
        });
        if let Some(operation) = sources {
            self.track(operation);
        }
    }
    fn finish(
        &self,
        remaining: &Cell<u8>,
        reply: Option<&async_channel::Sender<Result<Snapshot, String>>>,
    ) {
        remaining.set(remaining.get().saturating_sub(1));
        if remaining.get() == 0 {
            let snapshot = self.staging.borrow().clone();
            self.snapshot.replace(snapshot.clone());
            let _ = self.events.try_send(snapshot.clone());
            self.responses.borrow_mut().retain(|response| {
                if response.reply.is_closed() {
                    false
                } else if response.expected.matches(&snapshot) {
                    let _ = response.reply.try_send(Ok(snapshot.clone()));
                    false
                } else {
                    true
                }
            });
            if let Some(reply) = reply {
                let _ = reply.try_send(Ok(snapshot));
            }
        }
    }
    fn change(
        self: &Rc<Self>,
        change: Change,
        reply: async_channel::Sender<Result<Snapshot, String>>,
    ) {
        let snapshot = self.snapshot.borrow().clone();
        if !snapshot.available {
            let _ = reply.try_send(Err(gettext("The audio service is unavailable.")));
            return;
        }
        let device = if let Change::DefaultSink(name) = &change {
            snapshot.sinks.get(name)
        } else if matches!(change, Change::Microphone) {
            snapshot
                .source
                .as_ref()
                .and_then(|name| snapshot.sources.get(name))
        } else {
            snapshot
                .sink
                .as_ref()
                .and_then(|name| snapshot.sinks.get(name))
        };
        let Some(device) = device else {
            let _ = reply.try_send(Err(gettext("No audio device is available.")));
            return;
        };
        let expected = match &change {
            Change::Level(level) => Expected::Level(device.name.clone(), level.clamp(0.0, 1.0)),
            Change::Step(step) => {
                Expected::Level(device.name.clone(), (device.level + step).clamp(0.0, 1.0))
            }
            Change::Mute => Expected::Mute(device.name.clone(), !device.muted),
            Change::Microphone => Expected::Microphone(device.name.clone(), !device.muted),
            Change::DefaultSink(name) => Expected::Sink(name.clone()),
        };
        let weak = Rc::downgrade(self);
        let failure = reply.clone();
        let generation = self.epoch.get();
        let callback = Box::new(move |success| {
            if let Some(worker) = weak.upgrade() {
                if worker.epoch.get() != generation {
                    let _ = failure.try_send(Err(gettext(
                        "The audio device changed while the operation was pending.",
                    )));
                    return;
                }
                if success {
                    worker.refresh(Some(Response {
                        expected: expected.clone(),
                        reply: failure.clone(),
                    }));
                } else {
                    let _ = failure.try_send(Err(gettext("The audio operation was rejected.")));
                    worker.refresh(None);
                }
            }
        });
        let mut context = self.context.borrow_mut();
        let Some(context) = context.as_mut() else {
            return;
        };
        let mut introspect = context.introspect();
        let operation = match change {
            Change::Level(level) | Change::Step(level) => {
                let target = if matches!(change, Change::Step(_)) {
                    device.level + level
                } else {
                    level
                };
                if !target.is_finite() {
                    let _ = reply.try_send(Err(gettext("Invalid audio level.")));
                    return;
                }
                let mut volume = pulse::volume::ChannelVolumes::default();
                volume.set(
                    device.channels,
                    pulse::volume::Volume(
                        (target.clamp(0.0, 1.0) * f64::from(pulse::volume::Volume::NORMAL.0))
                            .round() as u32,
                    ),
                );
                introspect.set_sink_volume_by_name(&device.name, &volume, Some(callback))
            }
            Change::Mute => {
                introspect.set_sink_mute_by_name(&device.name, !device.muted, Some(callback))
            }
            Change::Microphone => {
                introspect.set_source_mute_by_name(&device.name, !device.muted, Some(callback))
            }
            Change::DefaultSink(name) => {
                if name.contains('\0') || !snapshot.sinks.contains_key(&name) {
                    let _ =
                        reply.try_send(Err(gettext("The selected audio device is unavailable.")));
                    return;
                }
                context.set_default_sink(&name, callback)
            }
        };
        self.track(operation);
    }
}
pub fn start(
    events: async_channel::Sender<Snapshot>,
) -> Result<async_channel::Sender<Work>, std::io::Error> {
    let (commands, receiver) = async_channel::unbounded();
    let internal = commands.clone();
    let server = std::env::var("PULSE_SERVER").ok();
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let socket = server
        .as_deref()
        .and_then(|s| s.strip_prefix("unix:"))
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime.join("pulse/native"));
    std::thread::Builder::new()
        .name("asena-audio".into())
        .spawn(move || {
            let mut context = glib::MainContext::new();
            let Some(native) = libpulse_glib_binding::Mainloop::new(Some(&mut context)) else {
                return;
            };
            let result = context.with_thread_default(|| {
                let main_loop = glib::MainLoop::new(Some(&context), false);
                let worker = Rc::new(Worker {
                    native,
                    context: RefCell::default(),
                    commands: internal.clone(),
                    events,
                    snapshot: RefCell::default(),
                    staging: RefCell::default(),
                    generation: Cell::new(0),
                    epoch: Cell::new(0),
                    responses: RefCell::default(),
                    pending: RefCell::default(),
                    server,
                });
                let mut monitors = Vec::new();
                for path in [
                    socket.clone(),
                    socket.parent().unwrap_or(&runtime).to_path_buf(),
                    runtime,
                ] {
                    if path.as_os_str().is_empty() {
                        continue;
                    }
                    let file = gio::File::for_path(&path);
                    let monitor = if path.is_dir() {
                        file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
                    } else {
                        file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
                    };
                    if let Ok(monitor) = monitor {
                        let commands = internal.clone();
                        monitor.connect_changed(move |_, _, _, _| {
                            let _ = commands.try_send(Work::Connect);
                        });
                        monitors.push(monitor);
                    }
                }
                let finished = main_loop.clone();
                let active = worker.clone();
                context.spawn_local(async move {
                    while let Ok(work) = receiver.recv().await {
                        match work {
                            Work::Connect => active.connect(),
                            Work::State => active.state(),
                            Work::Refresh => active.refresh(None),
                            Work::Request(change, reply) => active.change(change, reply),
                            Work::Stop => break,
                        }
                    }
                    active.disconnect();
                    finished.quit();
                });
                worker.connect();
                main_loop.run();
                for monitor in monitors {
                    monitor.cancel();
                }
            });
            if let Err(error) = result {
                tracing::warn!(%error,"Audio worker context failed");
            }
        })?;
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledgement_requires_the_observed_target() {
        let mut snapshot = Snapshot {
            available: true,
            sink: Some("test".into()),
            ..Default::default()
        };
        snapshot.sinks.insert(
            "test".into(),
            Device {
                name: "test".into(),
                description: "test".into(),
                channels: 2,
                level: 1.0,
                muted: false,
            },
        );
        let expected = Expected::Level("test".into(), 0.3);
        assert!(!expected.matches(&snapshot));
        if let Some(device) = snapshot.sinks.get_mut("test") {
            device.level = 0.300003;
        }
        assert!(expected.matches(&snapshot));
        assert!(!Expected::Mute("test".into(), true).matches(&snapshot));
        assert!(!Expected::Level("missing".into(), 0.3).matches(&snapshot));
    }
}
