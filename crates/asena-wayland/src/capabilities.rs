//! One registry roundtrip. Call on a worker, never from the GTK context.
use std::collections::BTreeMap;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};
#[derive(Default)]
struct Inventory(BTreeMap<String, u32>);
impl Dispatch<wl_registry::WlRegistry, ()> for Inventory {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            interface, version, ..
        } = event
        {
            state.0.insert(interface, version);
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not connect to Wayland: {0}")]
    Connect(#[from] wayland_client::ConnectError),
    #[error("Could not read Wayland globals: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),
}
pub fn globals() -> Result<BTreeMap<String, u32>, Error> {
    let connection = Connection::connect_to_env()?;
    let mut queue = connection.new_event_queue();
    let _registry = connection.display().get_registry(&queue.handle(), ());
    let mut state = Inventory::default();
    queue.roundtrip(&mut state)?;
    Ok(state.0)
}
