//! Worker-only, bounded output-power requests on a separate Wayland connection.
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    os::fd::AsFd,
    time::Duration,
};
use wayland_client::{
    protocol::{wl_callback, wl_output, wl_registry},
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
};
use wayland_protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1,
    zwlr_output_power_v1::{self, Mode, ZwlrOutputPowerV1},
};
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not connect to Wayland: {0}")]
    Connect(#[from] wayland_client::ConnectError),
    #[error("Wayland dispatch failed: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),
    #[error("Wayland IO failed: {0}")]
    Io(#[from] io::Error),
    #[error("Output power control is unavailable")]
    Unavailable,
    #[error("Output power control timed out")]
    Timeout,
    #[error("Too many outputs for a bounded power operation")]
    TooMany,
}
#[derive(Default)]
struct State {
    manager: Option<(u32, ZwlrOutputPowerManagerV1)>,
    outputs: BTreeMap<u32, wl_output::WlOutput>,
    powers: BTreeMap<u32, (ZwlrOutputPowerV1, Option<Mode>)>,
    barrier: bool,
    failed: bool,
    overflow: bool,
    removed: BTreeSet<u32>,
}
pub struct Session {
    _registry: wl_registry::WlRegistry,
    connection: Connection,
    queue: EventQueue<State>,
    socket: async_io::Async<std::os::fd::OwnedFd>,
    state: State,
}
async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T, Error>>,
) -> Result<T, Error> {
    futures_lite::future::race(future, async {
        async_io::Timer::after(Duration::from_secs(3)).await;
        Err(Error::Timeout)
    })
    .await
}
impl Session {
    /// Worker-only. Keep this connection between screen-off and restore: disabling
    /// an output can remove its registry global while its existing proxy stays valid.
    pub fn connect() -> Result<Self, Error> {
        futures_lite::future::block_on(bounded(async {
            let connection = Connection::connect_to_env()?;
            let mut queue = connection.new_event_queue::<State>();
            let _registry = connection.display().get_registry(&queue.handle(), ());
            let mut state = State::default();
            let socket = async_io::Async::new(connection.as_fd().try_clone_to_owned()?)?;
            sync(&connection, &mut queue, &socket, &mut state).await?;
            Ok(Self {
                _registry,
                connection,
                queue,
                socket,
                state,
            })
        }))
    }
    pub fn set_power(&mut self, enabled: bool) -> Result<(), Error> {
        futures_lite::future::block_on(bounded(self.apply(enabled)))
    }
    async fn apply(&mut self, enabled: bool) -> Result<(), Error> {
        let Self {
            connection,
            queue,
            socket,
            state,
            ..
        } = self;
        state.failed = false;
        sync(connection, queue, socket, state).await?;
        let desired = if enabled { Mode::On } else { Mode::Off };
        if enabled && !state.powers.is_empty() {
            // Restore the controls retained before DPMS. Re-added globals can alias
            // these same outputs and must not request a second exclusive control.
            for (power, _) in state.powers.values() {
                power.set_mode(desired);
            }
            sync(connection, queue, socket, state).await?;
            if !state.failed
                && !state.powers.is_empty()
                && state
                    .powers
                    .values()
                    .all(|(_, mode)| *mode == Some(desired))
            {
                return Ok(());
            }
            return Err(Error::Unavailable);
        }
        let manager = state
            .manager
            .as_ref()
            .map(|(_, manager)| manager.clone())
            .ok_or(Error::Unavailable)?;
        let handle = queue.handle();
        for _ in 0..3 {
            if state.overflow {
                return Err(Error::TooMany);
            }
            if state.outputs.is_empty() {
                return Err(Error::Unavailable);
            }
            for (name, output) in state.outputs.clone() {
                state
                    .powers
                    .entry(name)
                    .or_insert_with(|| (manager.get_output_power(&output, &handle, name), None));
            }
            sync(connection, queue, socket, state).await?;
            if state.failed {
                return Err(Error::Unavailable);
            }
            for (power, mode) in state.powers.values() {
                if *mode != Some(desired) {
                    power.set_mode(desired);
                }
            }
            sync(connection, queue, socket, state).await?;
            if state.failed {
                return Err(Error::Unavailable);
            }
            if !state.outputs.is_empty()
                && state.outputs.keys().all(|name| {
                    state
                        .powers
                        .get(name)
                        .is_some_and(|(_, mode)| *mode == Some(desired))
                })
            {
                return Ok(());
            }
        }
        Err(Error::Unavailable)
    }
}
async fn sync(
    connection: &Connection,
    queue: &mut EventQueue<State>,
    socket: &async_io::Async<std::os::fd::OwnedFd>,
    state: &mut State,
) -> Result<(), Error> {
    state.barrier = false;
    connection.display().sync(&queue.handle(), ());
    while !state.barrier {
        queue.dispatch_pending(state)?;
        if state.barrier {
            break;
        }
        socket
            .write_with(|_| connection.flush().map_err(to_io))
            .await?;
        let read = socket
            .read_with(|_| match connection.prepare_read() {
                Some(guard) => guard.read().map_err(to_io),
                None => Ok(0),
            })
            .await?;
        if read == 0 {
            futures_lite::future::yield_now().await;
        }
    }
    Ok(())
}
fn to_io(error: wayland_client::backend::WaylandError) -> io::Error {
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
        handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "zwlr_output_power_manager_v1" => {
                    state.manager = Some((name, registry.bind(name, version.min(1), handle, ())))
                }
                "wl_output" => {
                    if state.outputs.len() >= 32 {
                        state.overflow = true;
                    } else {
                        state
                            .outputs
                            .insert(name, registry.bind(name, version.min(4), handle, ()));
                    }
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                state.removed.insert(name);
                if !state.powers.contains_key(&name) {
                    state.outputs.remove(&name);
                }
                if state.manager.as_ref().is_some_and(|(id, _)| *id == name) {
                    state.manager = None;
                    state.failed = true;
                }
            }
            _ => {}
        }
    }
}
impl Dispatch<wl_callback::WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_callback::Event::Done { .. }) {
            state.barrier = true;
        }
    }
}
impl Dispatch<ZwlrOutputPowerV1, u32> for State {
    fn event(
        state: &mut Self,
        power: &ZwlrOutputPowerV1,
        event: zwlr_output_power_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_output_power_v1::Event::Failed = event {
            power.destroy();
            if state
                .powers
                .get(name)
                .is_some_and(|(current, _)| current == power)
            {
                state.powers.remove(name);
                if state.removed.contains(name) {
                    state.outputs.remove(name);
                } else {
                    state.failed = true;
                }
            }
            return;
        }
        if let zwlr_output_power_v1::Event::Mode {
            mode: WEnum::Value(mode),
        } = event
        {
            if let Some((current, value)) = state.powers.get_mut(name) {
                if current == power {
                    *value = Some(mode);
                }
            }
        }
    }
}
wayland_client::delegate_noop!(State:ignore wl_output::WlOutput);
wayland_client::delegate_noop!(State:ignore ZwlrOutputPowerManagerV1);
