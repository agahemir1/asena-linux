//! wlr-output-management on its own connection and worker thread. Reports the
//! heads after every atomic update and tests or applies configurations.
use std::{collections::BTreeMap, io, os::fd::AsFd};

use async_channel::{Receiver, Sender};
use wayland_client::{
    protocol::{wl_output, wl_registry},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_configuration_v1::{self as configuration, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::{self as head, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self as manager, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self as mode, ZwlrOutputModeV1},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    /// Millihertz; zero when unknown.
    pub refresh: i32,
    pub preferred: bool,
}

impl Mode {
    fn same(&self, other: &Mode) -> bool {
        self.width == other.width && self.height == other.height && self.refresh == other.refresh
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Head {
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub modes: Vec<Mode>,
    pub enabled: bool,
    pub mode: Option<Mode>,
    pub x: i32,
    pub y: i32,
    pub transform: u32,
    pub scale: f64,
}

/// Desired state of one head. Heads left out keep their current state.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadConfig {
    pub name: String,
    pub enabled: bool,
    pub mode: Option<Mode>,
    pub x: i32,
    pub y: i32,
    pub transform: u32,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed,
    /// The heads changed before the compositor handled the request.
    Cancelled,
}

#[derive(Clone, Debug)]
pub enum Event {
    Available(bool),
    Heads(Vec<Head>),
}

pub struct Request {
    pub heads: Vec<HeadConfig>,
    /// Ask whether the configuration would work without applying it.
    pub test: bool,
    pub reply: Sender<Outcome>,
}

pub struct Bridge {
    pub requests: Sender<Request>,
    pub events: Receiver<Event>,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.requests.close();
    }
}

pub fn start() -> io::Result<Bridge> {
    let (requests, incoming) = async_channel::unbounded();
    let (outgoing, events) = async_channel::unbounded();
    std::thread::Builder::new()
        .name("asena-outputs".into())
        .spawn(move || {
            if let Err(error) = async_io::block_on(run(incoming, outgoing.clone())) {
                tracing::warn!(%error, "Output management connection stopped");
            }
            let _ = outgoing.try_send(Event::Available(false));
        })?;
    Ok(Bridge { requests, events })
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("{0}")]
    Connect(#[from] wayland_client::ConnectError),
    #[error("{0}")]
    Dispatch(#[from] wayland_client::DispatchError),
    #[error("{0}")]
    Io(#[from] io::Error),
}

struct HeadState {
    proxy: ZwlrOutputHeadV1,
    head: Head,
    modes: Vec<u32>,
    current: Option<u32>,
}

struct State {
    manager: Option<(u32, ZwlrOutputManagerV1)>,
    serial: Option<u32>,
    heads: BTreeMap<u32, HeadState>,
    /// Head ids in arrival order, so outputs keep a stable order.
    order: Vec<u32>,
    modes: BTreeMap<u32, (ZwlrOutputModeV1, Mode)>,
    pending: Option<(ZwlrOutputConfigurationV1, Sender<Outcome>)>,
    events: Sender<Event>,
    finished: bool,
}

impl State {
    fn snapshot(&self) -> Vec<Head> {
        self.order
            .iter()
            .filter_map(|id| self.heads.get(id))
            .map(|state| Head {
                modes: state
                    .modes
                    .iter()
                    .filter_map(|id| self.modes.get(id).map(|(_, mode)| *mode))
                    .collect(),
                mode: state
                    .current
                    .and_then(|id| self.modes.get(&id).map(|(_, mode)| *mode)),
                ..state.head.clone()
            })
            .collect()
    }

    fn configure(&mut self, request: Request, qh: &QueueHandle<Self>) {
        let (Some((_, manager)), Some(serial)) = (&self.manager, self.serial) else {
            let _ = request.reply.try_send(Outcome::Failed);
            return;
        };
        if self.pending.is_some() {
            let _ = request.reply.try_send(Outcome::Failed);
            return;
        }
        let configuration = manager.create_configuration(serial, qh, ());
        // The protocol requires every head to be enabled or disabled explicitly.
        for id in &self.order {
            let Some(state) = self.heads.get(id) else {
                continue;
            };
            let current = &state.head;
            let Some(wanted) = request
                .heads
                .iter()
                .find(|wanted| wanted.name == current.name)
            else {
                if current.enabled {
                    configuration.enable_head(&state.proxy, qh, ());
                } else {
                    configuration.disable_head(&state.proxy);
                }
                continue;
            };
            if !wanted.enabled {
                configuration.disable_head(&state.proxy);
                continue;
            }
            let head = configuration.enable_head(&state.proxy, qh, ());
            // The compositor starts from the head's current state; send only
            // what changes, and everything when the head was off.
            let fresh = !current.enabled;
            let current_mode = state
                .current
                .and_then(|id| self.modes.get(&id).map(|(_, mode)| *mode));
            if let Some(mode) = wanted.mode {
                if fresh || !current_mode.is_some_and(|current| current.same(&mode)) {
                    match state.modes.iter().find_map(|id| {
                        self.modes
                            .get(id)
                            .filter(|(_, known)| known.same(&mode))
                            .map(|(proxy, _)| proxy)
                    }) {
                        Some(proxy) => head.set_mode(proxy),
                        None => head.set_custom_mode(mode.width, mode.height, mode.refresh),
                    }
                }
            }
            if fresh || (wanted.x, wanted.y) != (current.x, current.y) {
                head.set_position(wanted.x, wanted.y);
            }
            if fresh || wanted.transform != current.transform {
                if let Ok(transform) = wl_output::Transform::try_from(wanted.transform) {
                    head.set_transform(transform);
                }
            }
            if fresh || (wanted.scale - current.scale).abs() > 1e-6 {
                head.set_scale(wanted.scale);
            }
        }
        if request.test {
            configuration.test();
        } else {
            configuration.apply();
        }
        self.pending = Some((configuration, request.reply));
    }

    fn remove_head(&mut self, id: u32) {
        if let Some(state) = self.heads.remove(&id) {
            for mode in state.modes {
                if let Some((proxy, _)) = self.modes.remove(&mode) {
                    if proxy.version() >= 3 {
                        proxy.release();
                    }
                }
            }
            if state.proxy.version() >= 3 {
                state.proxy.release();
            }
        }
        self.order.retain(|known| *known != id);
    }
}

async fn run(requests: Receiver<Request>, events: Sender<Event>) -> Result<(), Error> {
    let connection = Connection::connect_to_env()?;
    let mut queue = connection.new_event_queue::<State>();
    let qh = queue.handle();
    let _registry = connection.display().get_registry(&qh, ());
    let mut state = State {
        manager: None,
        serial: None,
        heads: BTreeMap::new(),
        order: Vec::new(),
        modes: BTreeMap::new(),
        pending: None,
        events,
        finished: false,
    };
    queue.roundtrip(&mut state)?;
    if state.manager.is_none() {
        tracing::warn!("Compositor does not advertise output management");
        return Ok(());
    }
    let _ = state.events.try_send(Event::Available(true));
    let socket = async_io::Async::new(connection.as_fd().try_clone_to_owned()?)?;
    enum Wake {
        Command(Result<Request, async_channel::RecvError>),
        Read(io::Result<usize>),
    }
    while !state.finished {
        queue.dispatch_pending(&mut state)?;
        if state.finished {
            break;
        }
        socket
            .write_with(|_| connection.flush().map_err(to_io_error))
            .await?;
        let read = async {
            socket
                .read_with(|_| match connection.prepare_read() {
                    Some(guard) => guard.read().map_err(to_io_error),
                    None => Ok(0),
                })
                .await
        };
        match futures_lite::future::race(async { Wake::Command(requests.recv().await) }, async {
            Wake::Read(read.await)
        })
        .await
        {
            Wake::Command(Ok(request)) => state.configure(request, &qh),
            Wake::Command(Err(_)) => break,
            Wake::Read(result) => {
                result?;
            }
        }
    }
    if let Some((_, reply)) = state.pending.take() {
        let _ = reply.try_send(Outcome::Failed);
    }
    Ok(())
}

fn to_io_error(error: wayland_client::backend::WaylandError) -> io::Error {
    match error {
        wayland_client::backend::WaylandError::Io(error) => error,
        error => io::Error::other(error),
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == "zwlr_output_manager_v1" && state.manager.is_none() => {
                // Version 4 only adds adaptive sync; mode objects stop at version 3.
                state.manager = Some((name, registry.bind(name, version.min(3), qh, ())));
            }
            wl_registry::Event::GlobalRemove { name }
                if state
                    .manager
                    .as_ref()
                    .is_some_and(|(global, _)| *global == name) =>
            {
                state.finished = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZwlrOutputManagerV1,
        event: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            manager::Event::Head { head } => {
                let id = head.id().protocol_id();
                state.order.push(id);
                state.heads.insert(
                    id,
                    HeadState {
                        proxy: head,
                        head: Head {
                            scale: 1.0,
                            ..Head::default()
                        },
                        modes: Vec::new(),
                        current: None,
                    },
                );
            }
            manager::Event::Done { serial } => {
                state.serial = Some(serial);
                let _ = state.events.try_send(Event::Heads(state.snapshot()));
            }
            manager::Event::Finished => state.finished = true,
            _ => {}
        }
    }
    wayland_client::event_created_child!(State, ZwlrOutputManagerV1, [
        manager::EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ())
    ]);
}

impl Dispatch<ZwlrOutputHeadV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: head::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = proxy.id().protocol_id();
        if let head::Event::Finished = event {
            state.remove_head(id);
            return;
        }
        if let head::Event::Mode { mode } = &event {
            state
                .modes
                .insert(mode.id().protocol_id(), (mode.clone(), Mode::default()));
        }
        let Some(entry) = state.heads.get_mut(&id) else {
            return;
        };
        let head = &mut entry.head;
        match event {
            head::Event::Name { name } => head.name = name,
            head::Event::Description { description } => head.description = description,
            head::Event::Mode { mode } => entry.modes.push(mode.id().protocol_id()),
            head::Event::Enabled { enabled } => head.enabled = enabled != 0,
            head::Event::CurrentMode { mode } => entry.current = Some(mode.id().protocol_id()),
            head::Event::Position { x, y } => (head.x, head.y) = (x, y),
            head::Event::Transform {
                transform: WEnum::Value(transform),
            } => head.transform = transform.into(),
            head::Event::Scale { scale } => head.scale = scale,
            head::Event::Make { make } => head.make = make,
            head::Event::Model { model } => head.model = model,
            head::Event::SerialNumber { serial_number } => head.serial = serial_number,
            _ => {}
        }
    }
    wayland_client::event_created_child!(State, ZwlrOutputHeadV1, [
        head::EVT_MODE_OPCODE => (ZwlrOutputModeV1, ())
    ]);
}

impl Dispatch<ZwlrOutputModeV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: mode::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = proxy.id().protocol_id();
        if let mode::Event::Finished = event {
            if let Some((proxy, _)) = state.modes.remove(&id) {
                if proxy.version() >= 3 {
                    proxy.release();
                }
            }
            for head in state.heads.values_mut() {
                head.modes.retain(|known| *known != id);
                if head.current == Some(id) {
                    head.current = None;
                }
            }
            return;
        }
        let Some((_, mode)) = state.modes.get_mut(&id) else {
            return;
        };
        match event {
            mode::Event::Size { width, height } => (mode.width, mode.height) = (width, height),
            mode::Event::Refresh { refresh } => mode.refresh = refresh,
            mode::Event::Preferred => mode.preferred = true,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputConfigurationV1,
        event: configuration::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let outcome = match event {
            configuration::Event::Succeeded => Outcome::Succeeded,
            configuration::Event::Failed => Outcome::Failed,
            configuration::Event::Cancelled => Outcome::Cancelled,
            _ => return,
        };
        if state
            .pending
            .as_ref()
            .is_some_and(|(pending, _)| pending == proxy)
        {
            if let Some((configuration, reply)) = state.pending.take() {
                configuration.destroy();
                let _ = reply.try_send(outcome);
            }
        }
    }
}

wayland_client::delegate_noop!(State: ignore ZwlrOutputConfigurationHeadV1);
