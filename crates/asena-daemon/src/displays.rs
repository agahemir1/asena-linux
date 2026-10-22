//! Sole owner of display layout changes: validate, test, apply, wait for
//! confirmation and revert. Confirmed layouts are saved per set of monitors
//! and applied again when exactly those monitors are connected.
use asena_core::{
    displays::{self, Config, Identity, Layout, Output, Saved},
    i18n::gettext,
};
use asena_wayland::output::{self as wayland, Outcome};
use async_channel::{Receiver, Sender};
use futures_lite::StreamExt;
use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const FILE_NAME: &str = "displays.json";

pub enum Command {
    Apply {
        configs: Vec<Config>,
        caller: String,
        reply: Sender<Result<u32, String>>,
    },
    Confirm(Sender<Result<(), String>>),
    Revert(Sender<Result<(), String>>),
    CallerGone(String),
}

/// The latest layout, read by the D-Bus thread.
#[derive(Default)]
pub struct Status(Mutex<Layout>);

impl Status {
    pub fn json(&self) -> String {
        self.0
            .lock()
            .map(|layout| layout.to_json())
            .unwrap_or_default()
    }

    fn set(&self, layout: Layout) {
        if let Ok(mut current) = self.0.lock() {
            *current = layout;
        }
    }
}

struct Pending {
    previous: Vec<(Identity, Config)>,
    caller: String,
    deadline: Instant,
    watcher: glib::JoinHandle<()>,
}

/// The nested development session keeps layouts beside its disposable
/// compositor configuration; a real session uses the user's configuration.
fn storage() -> Option<PathBuf> {
    if std::env::var_os("ASENA_DEV").is_some() {
        return std::env::var_os("ASENA_DEV_CONFIG")
            .map(PathBuf::from)
            .and_then(|path| path.parent().map(|parent| parent.join(FILE_NAME)));
    }
    Some(glib::user_config_dir().join("asena").join(FILE_NAME))
}

fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::other("The layout path has no directory"))?;
    std::fs::create_dir_all(directory)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_nanos();
    let temporary = directory.join(format!(".{FILE_NAME}.{stamp}.tmp"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    std::fs::rename(temporary, path)
}

fn output(head: wayland::Head) -> Output {
    let mode = |mode: wayland::Mode| displays::Mode {
        width: mode.width,
        height: mode.height,
        refresh: mode.refresh,
        preferred: mode.preferred,
    };
    Output {
        description: head.description,
        make: head.make,
        model: head.model,
        serial: head.serial,
        modes: head.modes.into_iter().map(mode).collect(),
        state: Config {
            name: head.name,
            enabled: head.enabled,
            mode: head.mode.map(mode),
            x: head.x,
            y: head.y,
            transform: head.transform,
            scale: head.scale,
        },
    }
}

fn head_config(config: &Config) -> wayland::HeadConfig {
    wayland::HeadConfig {
        name: config.name.clone(),
        enabled: config.enabled,
        mode: config.mode.map(|mode| wayland::Mode {
            width: mode.width,
            height: mode.height,
            refresh: mode.refresh,
            preferred: mode.preferred,
        }),
        x: config.x,
        y: config.y,
        transform: config.transform,
        scale: config.scale,
    }
}

async fn configure(bridge: &wayland::Bridge, configs: &[Config], test: bool) -> Outcome {
    let (reply, outcome) = async_channel::bounded(1);
    let request = wayland::Request {
        heads: configs.iter().map(head_config).collect(),
        test,
        reply,
    };
    if bridge.requests.send(request).await.is_err() {
        return Outcome::Failed;
    }
    futures_lite::future::race(
        async { outcome.recv().await.unwrap_or(Outcome::Failed) },
        async {
            async_io::Timer::after(Duration::from_secs(5)).await;
            Outcome::Failed
        },
    )
    .await
}

/// Tests, then applies. Our own validation comes first: some compositors
/// accept every test request.
async fn test_and_apply(bridge: &wayland::Bridge, configs: &[Config]) -> Result<(), String> {
    let refused = |outcome| match outcome {
        Outcome::Cancelled => gettext("The connected displays changed. Review the layout again."),
        _ => gettext("The displays do not support this layout."),
    };
    match configure(bridge, configs, true).await {
        Outcome::Succeeded => {}
        outcome => return Err(refused(outcome)),
    }
    match configure(bridge, configs, false).await {
        Outcome::Succeeded => Ok(()),
        outcome => Err(refused(outcome)),
    }
}

async fn notify(connection: &zbus::Connection) {
    let _ = connection
        .emit_signal(
            None::<&str>,
            asena_core::dbus::DAEMON_PATH,
            asena_core::dbus::DAEMON_ID,
            "DisplaysChanged",
            &(),
        )
        .await;
}

/// Reports `caller` once it leaves the bus, as when Settings closes or crashes.
fn watch(
    connection: &zbus::Connection,
    caller: String,
    sender: Sender<Command>,
) -> glib::JoinHandle<()> {
    let connection = connection.clone();
    glib::spawn_future_local(async move {
        let gone = async {
            let bus = zbus::fdo::DBusProxy::new(&connection).await?;
            let mut changes = bus
                .receive_name_owner_changed_with_args(&[(0, caller.as_str())])
                .await?;
            let name = zbus::names::BusName::try_from(caller.as_str())
                .map_err(|error| zbus::Error::Failure(error.to_string()))?;
            if !bus.name_has_owner(name).await? {
                return Ok::<_, zbus::Error>(());
            }
            while let Some(change) = changes.next().await {
                if change.args()?.new_owner().is_none() {
                    break;
                }
            }
            Ok(())
        }
        .await;
        if let Err(error) = gone {
            // Without a watch the deadline still reverts the change.
            tracing::warn!(%error, "Could not watch the display settings client");
            return;
        }
        let _ = sender.send(Command::CallerGone(caller)).await;
    })
}

struct Displays {
    bridge: Option<wayland::Bridge>,
    connection: zbus::Connection,
    status: Arc<Status>,
    sender: Sender<Command>,
    path: Option<PathBuf>,
    saved: Saved,
    available: bool,
    outputs: Vec<Output>,
    hardware: Option<BTreeSet<Identity>>,
    pending: Option<Pending>,
}

impl Displays {
    async fn publish(&self) {
        self.status.set(Layout {
            available: self.available,
            pending: self.pending.is_some(),
            outputs: self.outputs.clone(),
        });
        notify(&self.connection).await;
    }

    async fn heads(&mut self, heads: Vec<wayland::Head>) {
        self.outputs = heads.into_iter().map(output).collect();
        let hardware: BTreeSet<Identity> = self.outputs.iter().map(Output::identity).collect();
        let replugged = self.hardware.as_ref() != Some(&hardware);
        self.hardware = Some(hardware);
        if replugged && self.pending.is_some() {
            tracing::info!("Monitors changed during a display change; reverting");
            self.revert().await;
        } else if replugged {
            self.restore_saved().await;
        }
        self.publish().await;
    }

    /// A confirmed layout for exactly these monitors is applied without asking again.
    async fn restore_saved(&mut self) {
        let (Some(bridge), Some(configs)) = (&self.bridge, self.saved.layout_for(&self.outputs))
        else {
            return;
        };
        if displays::unchanged(&self.outputs, &configs) {
            return;
        }
        match test_and_apply(bridge, &configs).await {
            Ok(()) => tracing::info!("Applied the saved display layout"),
            Err(error) => tracing::warn!(%error, "Could not apply the saved display layout"),
        }
    }

    async fn apply(&mut self, configs: Vec<Config>, caller: String) -> Result<u32, String> {
        let Some(bridge) = self.bridge.as_ref().filter(|_| self.available) else {
            return Err(gettext(
                "This compositor does not support display settings.",
            ));
        };
        if self.pending.is_some() {
            return Err(gettext(
                "Another display change is waiting for confirmation.",
            ));
        }
        displays::validate(&self.outputs, &configs).map_err(|invalid| invalid.message())?;
        if displays::unchanged(&self.outputs, &configs) {
            return Ok(0);
        }
        let previous = displays::remembered(&self.outputs);
        if let Err(error) = test_and_apply(bridge, &configs).await {
            // A failed apply may leave part of the change behind.
            if let Some(restored) = displays::restore(&previous, &self.outputs) {
                let _ = configure(bridge, &restored, false).await;
            }
            return Err(error);
        }
        self.pending = Some(Pending {
            previous,
            watcher: watch(&self.connection, caller.clone(), self.sender.clone()),
            caller,
            deadline: Instant::now() + Duration::from_secs(displays::CONFIRM_SECONDS.into()),
        });
        self.publish().await;
        Ok(displays::CONFIRM_SECONDS)
    }

    async fn confirm(&mut self) -> Result<(), String> {
        let Some(pending) = self.pending.take() else {
            return Err(gettext("The display change was already reverted."));
        };
        pending.watcher.abort();
        self.saved
            .remember(&self.outputs, &displays::current(&self.outputs));
        if let Some(path) = self.path.clone() {
            let contents = self.saved.to_json();
            match gio::spawn_blocking(move || write_atomic(&path, &contents)).await {
                Ok(Ok(())) => {}
                result => tracing::warn!(?result, "Could not save the display layout"),
            }
        }
        self.publish().await;
        Ok(())
    }

    /// Restores the previous layout on the monitors that are still connected.
    async fn revert(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        pending.watcher.abort();
        if let (Some(bridge), Some(configs)) = (
            &self.bridge,
            displays::restore(&pending.previous, &self.outputs),
        ) {
            if !displays::unchanged(&self.outputs, &configs) {
                if let Outcome::Succeeded = configure(bridge, &configs, false).await {
                    tracing::info!("Reverted the display change");
                } else {
                    tracing::warn!("Could not revert the display change");
                }
            }
        }
        self.publish().await;
    }
}

pub async fn run(
    commands: Receiver<Command>,
    sender: Sender<Command>,
    status: Arc<Status>,
    connection: zbus::Connection,
) {
    let bridge = wayland::start()
        .map_err(|error| tracing::warn!(%error, "Output management is unavailable"))
        .ok();
    let path = storage();
    let saved = match path.clone() {
        Some(path) => gio::spawn_blocking(move || std::fs::read_to_string(path))
            .await
            .ok()
            .and_then(Result::ok)
            .map(|text| Saved::parse(&text))
            .unwrap_or_default(),
        None => Saved::default(),
    };
    let mut events = bridge.as_ref().map(|bridge| bridge.events.clone());
    let mut displays = Displays {
        bridge,
        connection,
        status,
        sender,
        path,
        saved,
        available: false,
        outputs: Vec::new(),
        hardware: None,
        pending: None,
    };
    enum Wake {
        Command(Option<Command>),
        Event(Option<wayland::Event>),
        Deadline,
    }
    loop {
        let deadline = displays.pending.as_ref().map(|pending| pending.deadline);
        let wake = futures_lite::future::race(
            async { Wake::Command(commands.recv().await.ok()) },
            futures_lite::future::race(
                async {
                    match &events {
                        Some(events) => Wake::Event(events.recv().await.ok()),
                        None => futures_lite::future::pending().await,
                    }
                },
                async {
                    match deadline {
                        Some(deadline) => {
                            async_io::Timer::at(deadline).await;
                            Wake::Deadline
                        }
                        None => futures_lite::future::pending().await,
                    }
                },
            ),
        )
        .await;
        match wake {
            Wake::Command(None) => break,
            Wake::Command(Some(Command::Apply {
                configs,
                caller,
                reply,
            })) => {
                let _ = reply.try_send(displays.apply(configs, caller).await);
            }
            Wake::Command(Some(Command::Confirm(reply))) => {
                let _ = reply.try_send(displays.confirm().await);
            }
            Wake::Command(Some(Command::Revert(reply))) => {
                displays.revert().await;
                let _ = reply.try_send(Ok(()));
            }
            Wake::Command(Some(Command::CallerGone(caller))) => {
                if displays
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.caller == caller)
                {
                    tracing::info!("Display settings closed before confirming; reverting");
                    displays.revert().await;
                }
            }
            Wake::Event(Some(wayland::Event::Available(available))) => {
                displays.available = available;
                displays.publish().await;
            }
            Wake::Event(Some(wayland::Event::Heads(heads))) => displays.heads(heads).await,
            Wake::Event(None) => {
                // The worker stopped; keep serving commands with clear errors.
                events = None;
                displays.bridge = None;
                displays.available = false;
                displays.publish().await;
            }
            Wake::Deadline => {
                tracing::info!("Display change not confirmed in time; reverting");
                displays.revert().await;
            }
        }
    }
}
