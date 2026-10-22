use std::{collections::BTreeMap, io, os::fd::AsFd};

use async_channel::{Receiver, Sender};
use wayland_client::{
    protocol::{wl_output, wl_registry, wl_seat},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self as handle, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self as manager, ZwlrForeignToplevelManagerV1},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub activated: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub outputs: Vec<String>,
}

impl Snapshot {
    fn set_states(&mut self, bytes: &[u8]) {
        self.maximized = false;
        self.minimized = false;
        self.activated = false;
        self.fullscreen = false;
        for chunk in bytes.chunks_exact(4) {
            let value = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            match value {
                0 => self.maximized = true,
                1 => self.minimized = true,
                2 => self.activated = true,
                3 => self.fullscreen = true,
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Available(bool),
    Updated(Snapshot),
    Closed(u64),
}

#[derive(Clone, Copy, Debug)]
pub enum Request {
    Activate(u64),
    Minimize(u64),
    Unminimize(u64),
    Close(u64),
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
        .name("asena-toplevel".into())
        .spawn(move || {
            // Blocking executor and initial roundtrips belong only to this worker.
            if let Err(error) = async_io::block_on(run(incoming, outgoing.clone())) {
                tracing::warn!(%error, "Toplevel connection stopped");
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

struct Window {
    handle: ZwlrForeignToplevelHandleV1,
    pending: Snapshot,
    output_ids: Vec<u32>,
}

struct State {
    manager: Option<(u32, ZwlrForeignToplevelManagerV1)>,
    seats: BTreeMap<u32, wl_seat::WlSeat>,
    outputs: BTreeMap<u32, (u32, wl_output::WlOutput, String)>,
    windows: BTreeMap<u32, Window>,
    next_id: u64,
    events: Sender<Event>,
    finished: bool,
}

impl State {
    fn request(&self, request: Request) {
        let id = match request {
            Request::Activate(id)
            | Request::Minimize(id)
            | Request::Unminimize(id)
            | Request::Close(id) => id,
        };
        let Some(window) = self.windows.values().find(|window| window.pending.id == id) else {
            return;
        };
        match request {
            Request::Activate(_) => {
                if let Some(seat) = self.seats.values().next() {
                    window.handle.unset_minimized();
                    window.handle.activate(seat);
                }
            }
            Request::Minimize(_) => window.handle.set_minimized(),
            Request::Unminimize(_) => window.handle.unset_minimized(),
            Request::Close(_) => window.handle.close(),
        }
    }
}

async fn run(requests: Receiver<Request>, events: Sender<Event>) -> Result<(), Error> {
    let connection = Connection::connect_to_env()?;
    let mut queue = connection.new_event_queue::<State>();
    let qh = queue.handle();
    let _registry = connection.display().get_registry(&qh, ());
    let mut state = State {
        manager: None,
        seats: BTreeMap::new(),
        outputs: BTreeMap::new(),
        windows: BTreeMap::new(),
        next_id: 0,
        events,
        finished: false,
    };
    queue.roundtrip(&mut state)?;
    if state.manager.is_none() {
        tracing::warn!("Compositor does not advertise foreign-toplevel management");
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
                .read_with(|_| {
                    // Never hold a read guard across an await. Cancellation leaves
                    // the socket ready for the next command/read cycle.
                    match connection.prepare_read() {
                        Some(guard) => guard.read().map_err(to_io_error),
                        None => Ok(0),
                    }
                })
                .await
        };
        match futures_lite::future::race(async { Wake::Command(requests.recv().await) }, async {
            Wake::Read(read.await)
        })
        .await
        {
            Wake::Command(Ok(request)) => state.request(request),
            Wake::Command(Err(_)) => break,
            Wake::Read(result) => {
                result?;
            }
        }
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
            } => match interface.as_str() {
                "zwlr_foreign_toplevel_manager_v1" if state.manager.is_none() => {
                    state.manager = Some((name, registry.bind(name, version.min(3), qh, ())));
                }
                "wl_seat" => {
                    state
                        .seats
                        .insert(name, registry.bind(name, version.min(5), qh, ()));
                }
                "wl_output" => {
                    let output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, ());
                    state
                        .outputs
                        .insert(output.id().protocol_id(), (name, output, String::new()));
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(seat) = state.seats.remove(&name) {
                    if seat.version() >= 5 {
                        seat.release();
                    }
                }
                state.outputs.retain(|_, (global, output, _)| {
                    if *global == name {
                        if output.version() >= 3 {
                            output.release();
                        }
                        false
                    } else {
                        true
                    }
                });
                if state
                    .manager
                    .as_ref()
                    .is_some_and(|(global, _)| *global == name)
                {
                    state.finished = true;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            manager::Event::Toplevel { toplevel } => {
                state.next_id += 1;
                state.windows.insert(
                    toplevel.id().protocol_id(),
                    Window {
                        handle: toplevel,
                        pending: Snapshot {
                            id: state.next_id,
                            ..Snapshot::default()
                        },
                        output_ids: vec![],
                    },
                );
            }
            manager::Event::Finished => {
                state.finished = true;
            }
            _ => {}
        }
    }
    wayland_client::event_created_child!(State, ZwlrForeignToplevelManagerV1, [0 => (ZwlrForeignToplevelHandleV1, ())]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let key = proxy.id().protocol_id();
        if let handle::Event::Closed = event {
            if let Some(window) = state.windows.remove(&key) {
                let _ = state.events.try_send(Event::Closed(window.pending.id));
            }
            proxy.destroy();
            return;
        }
        let Some(window) = state.windows.get_mut(&key) else {
            return;
        };
        match event {
            handle::Event::Title { title } => window.pending.title = title,
            handle::Event::AppId { app_id } => window.pending.app_id = app_id,
            handle::Event::State { state } => window.pending.set_states(&state),
            handle::Event::OutputEnter { output } => {
                let id = output.id().protocol_id();
                if !window.output_ids.contains(&id) {
                    window.output_ids.push(id);
                }
            }
            handle::Event::OutputLeave { output } => window
                .output_ids
                .retain(|id| *id != output.id().protocol_id()),
            handle::Event::Done => {
                window.pending.outputs = window
                    .output_ids
                    .iter()
                    .filter_map(|id| state.outputs.get(id))
                    .map(|(_, _, name)| name.clone())
                    .collect();
                let _ = state
                    .events
                    .try_send(Event::Updated(window.pending.clone()));
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            if let Some((_, _, stored)) = state.outputs.get_mut(&output.id().protocol_id()) {
                *stored = name;
            }
        }
    }
}
wayland_client::delegate_noop!(State: ignore wl_seat::WlSeat);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn states_replace_previous_values_and_ignore_unknown_values() {
        let mut snapshot = Snapshot::default();
        snapshot.set_states(
            &[0_u32, 2, 99]
                .into_iter()
                .flat_map(u32::to_ne_bytes)
                .collect::<Vec<_>>(),
        );
        assert!(snapshot.activated && snapshot.maximized);
        assert!(!snapshot.minimized && !snapshot.fullscreen);
        snapshot.set_states(
            &[1_u32, 3]
                .into_iter()
                .flat_map(u32::to_ne_bytes)
                .collect::<Vec<_>>(),
        );
        assert!(!snapshot.activated && !snapshot.maximized);
        assert!(snapshot.minimized && snapshot.fullscreen);
        snapshot.set_states(&[1, 2]);
        assert!(!snapshot.minimized && !snapshot.fullscreen);
    }
}
