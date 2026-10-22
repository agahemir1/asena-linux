//! Backlight sysfs reads stay on a worker; writes go through logind only.
use asena_core::{i18n::gettext, operation::OperationGate};
use gio::prelude::*;
use glib::subclass::{prelude::*, Signal};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    sync::OnceLock,
};
#[derive(Clone, Debug)]
struct Backlight {
    name: String,
    maximum: u32,
    current: u32,
    path: PathBuf,
}
fn read_backlight(root: &Path) -> Option<Backlight> {
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    paths.into_iter().find_map(|path| {
        let name = path.file_name()?.to_str()?.to_owned();
        let maximum = std::fs::read_to_string(path.join("max_brightness"))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?;
        if maximum == 0 {
            return None;
        }
        let current = std::fs::read_to_string(path.join("actual_brightness"))
            .or_else(|_| std::fs::read_to_string(path.join("brightness")))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?
            .min(maximum);
        Some(Backlight {
            name,
            maximum,
            current,
            path,
        })
    })
}
enum Work {
    Refresh(Option<async_channel::Sender<Option<Backlight>>>),
    Stop,
}
fn worker(
    events: async_channel::Sender<Option<Backlight>>,
) -> Result<async_channel::Sender<Work>, std::io::Error> {
    let (commands, receiver) = async_channel::unbounded();
    let internal = commands.clone();
    std::thread::Builder::new()
        .name("asena-backlight".into())
        .spawn(move || {
            let context = glib::MainContext::new();
            let result = context.with_thread_default(|| {
                let loop_ = glib::MainLoop::new(Some(&context), false);
                let finished = loop_.clone();
                context.spawn_local(async move {
                    let root = Path::new("/sys/class/backlight");
                    let mut monitors = Vec::<gio::FileMonitor>::new();
                    let mut watched = None::<String>;
                    let _ = internal.try_send(Work::Refresh(None));
                    while let Ok(work) = receiver.recv().await {
                        match work {
                            Work::Stop => break,
                            Work::Refresh(reply) => {
                                let value = read_backlight(root);
                                let name = value.as_ref().map(|value| value.name.clone());
                                if monitors.is_empty() || watched != name {
                                    for monitor in monitors.drain(..) {
                                        monitor.cancel();
                                    }
                                    let mut paths = vec![root.to_path_buf()];
                                    if let Some(value) = &value {
                                        paths.extend([
                                            value.path.join("actual_brightness"),
                                            value.path.join("brightness"),
                                        ]);
                                    }
                                    for path in paths {
                                        let file = gio::File::for_path(&path);
                                        let monitor = if path.is_dir() {
                                            file.monitor_directory(
                                                gio::FileMonitorFlags::NONE,
                                                gio::Cancellable::NONE,
                                            )
                                        } else {
                                            file.monitor_file(
                                                gio::FileMonitorFlags::NONE,
                                                gio::Cancellable::NONE,
                                            )
                                        };
                                        if let Ok(monitor) = monitor {
                                            let sender = internal.clone();
                                            monitor.connect_changed(move |_, _, _, _| {
                                                let _ = sender.try_send(Work::Refresh(None));
                                            });
                                            monitors.push(monitor);
                                        }
                                    }
                                    watched = name;
                                }
                                let _ = events.try_send(value.clone());
                                if let Some(reply) = reply {
                                    let _ = reply.try_send(value);
                                }
                            }
                        }
                    }
                    for monitor in monitors {
                        monitor.cancel();
                    }
                    finished.quit();
                });
                loop_.run();
            });
            if let Err(error) = result {
                tracing::warn!(%error,"Backlight context failed");
            }
        })?;
    Ok(commands)
}
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::BrightnessModel)]
    pub struct BrightnessModel {
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub level: Cell<f64>,
        #[property(get)]
        pub device_name: RefCell<String>,
        #[property(get)]
        pub busy: Cell<bool>,
        #[property(get)]
        pub last_error: RefCell<String>,
        pub(super) current: RefCell<Option<Backlight>>,
        pub(super) worker: RefCell<Option<async_channel::Sender<Work>>>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
        pub gate: RefCell<OperationGate>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for BrightnessModel {
        const NAME: &'static str = "AsenaBrightnessModel";
        type Type = super::BrightnessModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for BrightnessModel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
        fn dispose(&self) {
            if let Some(worker) = self.worker.borrow_mut().take() {
                let _ = worker.try_send(Work::Stop);
            }
            if let Some(task) = self.task.borrow_mut().take() {
                task.abort();
            }
        }
    }
}
glib::wrapper! {pub struct BrightnessModel(ObjectSubclass<imp::BrightnessModel>);}
impl Default for BrightnessModel {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl BrightnessModel {
    pub fn start(&self) -> Result<(), std::io::Error> {
        if self.imp().worker.borrow().is_some() {
            return Ok(());
        }
        let (events, receiver) = async_channel::unbounded();
        self.imp().worker.replace(Some(worker(events)?));
        let weak = self.downgrade();
        self.imp()
            .task
            .replace(Some(glib::spawn_future_local(async move {
                while let Ok(value) = receiver.recv().await {
                    let Some(model) = weak.upgrade() else {
                        break;
                    };
                    model.apply(value);
                }
            })));
        Ok(())
    }
    fn apply(&self, value: Option<Backlight>) {
        self.imp().device_name.replace(
            value
                .as_ref()
                .map(|value| value.name.clone())
                .unwrap_or_default(),
        );
        self.notify_device_name();
        let different = self.imp().current.borrow().as_ref().map(|v| &v.name)
            != value.as_ref().map(|v| &v.name);
        if different {
            self.imp().gate.borrow_mut().owner_changed(value.is_some());
            self.imp().busy.set(false);
            self.notify_busy();
        }
        self.imp().available.set(value.is_some());
        self.notify_available();
        self.imp().level.set(
            value
                .as_ref()
                .map(|v| f64::from(v.current) / f64::from(v.maximum))
                .unwrap_or(0.0),
        );
        self.notify_level();
        self.imp().current.replace(value);
        self.emit_by_name::<()>("changed", &[]);
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    /// Obtain fresh sysfs readback before restoring an idle dim; notifications
    /// alone are insufficient for drivers without brightness change events.
    pub async fn refresh(&self) -> Result<(), crate::error::ServiceError> {
        let worker = self
            .imp()
            .worker
            .borrow()
            .clone()
            .ok_or_else(|| gettext("No supported backlight is available."))?;
        let (reply, response) = async_channel::bounded(1);
        worker
            .send(Work::Refresh(Some(reply)))
            .await
            .map_err(|error| crate::error::ServiceError(error.to_string()))?;
        let value = response
            .recv()
            .await
            .map_err(|error| crate::error::ServiceError(error.to_string()))?;
        self.apply(value);
        Ok(())
    }
    pub async fn set_level(&self, level: f64) -> Result<f64, crate::error::ServiceError> {
        let value = self
            .imp()
            .current
            .borrow()
            .clone()
            .ok_or_else(|| gettext("No supported backlight is available."))?;
        let raw = asena_core::operation::brightness_raw(level, value.maximum)
            .ok_or_else(|| gettext("Invalid brightness level."))?;
        let ticket = self
            .imp()
            .gate
            .borrow_mut()
            .begin()
            .ok_or_else(|| gettext("The backlight is busy."))?;
        self.imp().busy.set(true);
        self.notify_busy();
        self.emit_by_name::<()>("changed", &[]);
        let result: Result<Option<Backlight>, String> = async {
            let connection = zbus::Connection::system()
                .await
                .map_err(|e| e.to_string())?;
            let proxy = zbus::Proxy::new(
                &connection,
                "org.freedesktop.login1",
                "/org/freedesktop/login1/session/auto",
                "org.freedesktop.login1.Session",
            )
            .await
            .map_err(|e| e.to_string())?;
            proxy
                .call::<_, _, ()>("SetBrightness", &("backlight", value.name, raw))
                .await
                .map_err(|e| e.to_string())?;
            let worker = self
                .imp()
                .worker
                .borrow()
                .clone()
                .ok_or_else(|| gettext("No supported backlight is available."))?;
            let (reply, response) = async_channel::bounded(1);
            worker
                .send(Work::Refresh(Some(reply)))
                .await
                .map_err(|e| e.to_string())?;
            response.recv().await.map_err(|e| e.to_string())
        }
        .await;
        if !self.imp().gate.borrow_mut().finish(ticket) {
            return Err(gettext("The backlight changed while the operation was pending.").into());
        }
        self.imp().busy.set(false);
        self.notify_busy();
        match result {
            Ok(value) => {
                self.apply(value);
                self.imp().last_error.replace(String::new());
                self.notify_last_error();
                Ok(self.level())
            }
            Err(error) => {
                self.imp().last_error.replace(error.clone());
                self.notify_last_error();
                self.emit_by_name::<()>("changed", &[]);
                Err(error.into())
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sysfs_fixture_rejects_zero_and_clamps_driver_values() -> Result<(), std::io::Error> {
        let root =
            std::env::temp_dir().join(format!("asena-backlight-test-{}", std::process::id()));
        let device = root.join("fixture");
        std::fs::create_dir_all(&device)?;
        std::fs::write(device.join("max_brightness"), "0")?;
        std::fs::write(device.join("actual_brightness"), "200")?;
        assert!(read_backlight(&root).is_none());
        std::fs::write(device.join("max_brightness"), "100")?;
        assert_eq!(
            read_backlight(&root).map(|v| (v.maximum, v.current)),
            Some((100, 100))
        );
        std::fs::remove_dir_all(root)
    }
}
