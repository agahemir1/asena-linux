use asena_core::i18n::gettext;
use futures_lite::io::AsyncBufReadExt;
use gio::prelude::*;
use std::{
    ffi::OsStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
#[derive(Default)]
pub struct Status {
    pub available: AtomicBool,
    pub active: AtomicBool,
    pub generation: AtomicU64,
    pub error: Mutex<String>,
}
impl Status {
    pub fn error(&self) -> String {
        self.error
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    }
    pub fn set_error(&self, error: String) {
        if let Ok(mut value) = self.error.lock() {
            *value = error;
        }
    }
}
pub enum Command {
    Apply(bool, u32),
    Stop,
}
#[derive(Debug)]
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}
pub async fn notify(connection: &zbus::Connection) {
    let _ = connection
        .emit_signal(
            None::<&str>,
            asena_core::dbus::DAEMON_PATH,
            asena_core::dbus::DAEMON_ID,
            "NightLightChanged",
            &(),
        )
        .await;
}
pub async fn run(
    receiver: async_channel::Receiver<Command>,
    status: Arc<Status>,
    connection: zbus::Connection,
    settings: gio::Settings,
) {
    let nested = std::env::var_os("ASENA_DEV").is_some();
    let supported = gio::spawn_blocking(move || {
        !nested
            && glib::find_program_in_path("wlsunset").is_some()
            && asena_wayland::capabilities::globals()
                .is_ok_and(|globals| globals.contains_key("zwlr_gamma_control_manager_v1"))
    })
    .await
    .unwrap_or(false);
    status.available.store(supported, Ordering::SeqCst);
    notify(&connection).await;
    let mut child = None::<Child>;
    let mut reader = None::<glib::JoinHandle<()>>;
    while let Ok(mut command) = receiver.recv().await {
        while let Ok(next) = receiver.try_recv() {
            if matches!(command, Command::Stop) {
                break;
            }
            command = next;
        }
        status.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(reader) = reader.take() {
            reader.abort();
        }
        if let Some(mut previous) = child.take() {
            let _ = gio::spawn_blocking(move || {
                let _ = previous.0.kill();
                previous.0.wait()
            })
            .await;
        }
        status.active.store(false, Ordering::SeqCst);
        notify(&connection).await;
        let Command::Apply(enabled, temperature) = command else {
            break;
        };
        if !enabled {
            continue;
        }
        if !supported {
            status.set_error(gettext("Night light is unavailable on this display."));
            let _ = settings.set_boolean("enabled", false);
            notify(&connection).await;
            continue;
        }
        let Some(arguments) = asena_core::night_light::arguments(temperature) else {
            status.set_error(gettext("Invalid night light temperature."));
            notify(&connection).await;
            continue;
        };
        let process = gio::spawn_blocking(move || {
            let arguments: Vec<_> = arguments.iter().map(OsStr::new).collect();
            std::process::Command::new("wlsunset")
                .args(arguments.iter().skip(1))
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map(Child)
        })
        .await;
        let mut process = match process {
            Ok(Ok(process)) => process,
            result => {
                tracing::warn!(?result, "Could not start night light");
                status.set_error(gettext("Night light could not be started."));
                notify(&connection).await;
                continue;
            }
        };
        status.set_error(String::new());
        let generation = status.generation.load(Ordering::SeqCst);
        if let Some(stream) = process.0.stderr.take() {
            let lines = async_io::Async::new(stream).map(futures_lite::io::BufReader::new);
            let Ok(mut lines) = lines else {
                let _ = gio::spawn_blocking(move || {
                    let _ = process.0.kill();
                    process.0.wait()
                })
                .await;
                continue;
            };
            let state = status.clone();
            let bus = connection.clone();
            let preferences = settings.clone();
            reader = Some(glib::spawn_future_local(async move {
                let mut outputs = asena_core::night_light::OutputState::default();
                loop {
                    let mut line = String::new();
                    match lines.read_line(&mut line).await {
                        Ok(0) | Err(_) => break,
                        _ => {}
                    }
                    let line = line.trim_end();
                    if state.generation.load(Ordering::SeqCst) != generation {
                        return;
                    }
                    outputs.line(line);
                    state.active.store(outputs.active(), Ordering::SeqCst);
                    if outputs.unsupported() {
                        state.available.store(false, Ordering::SeqCst);
                        state.set_error(gettext("Night light is unavailable on this display."));
                    } else if outputs.active() {
                        state.available.store(true, Ordering::SeqCst);
                        state.set_error(if outputs.partial() {
                            gettext("Night light is unavailable on some displays.")
                        } else {
                            String::new()
                        });
                    }
                    notify(&bus).await;
                }
                if state.generation.load(Ordering::SeqCst) == generation {
                    state.active.store(false, Ordering::SeqCst);
                    if preferences.boolean("enabled") {
                        state.set_error(gettext("Night light stopped."));
                        let _ = preferences.set_boolean("enabled", false);
                    }
                    notify(&bus).await;
                }
            }));
        }
        child = Some(process);
        let state = status.clone();
        let bus = connection.clone();
        let preferences = settings.clone();
        glib::spawn_future_local(async move {
            glib::timeout_future(std::time::Duration::from_secs(2)).await;
            if state.generation.load(Ordering::SeqCst) == generation
                && !state.active.load(Ordering::SeqCst)
                && preferences.boolean("enabled")
            {
                state.available.store(false, Ordering::SeqCst);
                state.set_error(gettext("Night light did not become ready."));
                let _ = preferences.set_boolean("enabled", false);
                notify(&bus).await;
            }
        });
    }
    status.generation.fetch_add(1, Ordering::SeqCst);
    if let Some(reader) = reader {
        reader.abort();
    }
    if let Some(mut child) = child {
        let _ = gio::spawn_blocking(move || {
            let _ = child.0.kill();
            child.0.wait()
        })
        .await;
    }
    status.active.store(false, Ordering::SeqCst);
}
