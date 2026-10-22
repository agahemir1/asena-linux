use crate::audio_worker::{self, Work};
pub use crate::audio_worker::{Change, Device, Snapshot};
use asena_core::{i18n::gettext, operation::OperationGate};
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
    time::Duration,
};
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::AudioModel)]
    pub struct AudioModel {
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub output_available: Cell<bool>,
        #[property(get)]
        pub microphone_available: Cell<bool>,
        #[property(get)]
        pub volume: Cell<f64>,
        #[property(get)]
        pub muted: Cell<bool>,
        #[property(get)]
        pub microphone_muted: Cell<bool>,
        #[property(get)]
        pub busy: Cell<bool>,
        #[property(get)]
        pub last_error: RefCell<String>,
        pub snapshot: RefCell<Snapshot>,
        pub worker: RefCell<Option<async_channel::Sender<Work>>>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
        pub gate: RefCell<OperationGate>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for AudioModel {
        const NAME: &'static str = "AsenaAudioModel";
        type Type = super::AudioModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for AudioModel {
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
glib::wrapper! {pub struct AudioModel(ObjectSubclass<imp::AudioModel>);}
impl Default for AudioModel {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl AudioModel {
    pub fn start(&self) -> Result<(), std::io::Error> {
        if self.imp().worker.borrow().is_some() {
            return Ok(());
        }
        let (events, receiver) = async_channel::unbounded();
        let worker = audio_worker::start(events)?;
        self.imp().worker.replace(Some(worker));
        let weak = self.downgrade();
        let task = glib::spawn_future_local(async move {
            while let Ok(snapshot) = receiver.recv().await {
                let Some(model) = weak.upgrade() else {
                    break;
                };
                model.apply(snapshot);
            }
        });
        self.imp().task.replace(Some(task));
        Ok(())
    }
    fn apply(&self, snapshot: Snapshot) {
        let changed = {
            let old = self.imp().snapshot.borrow();
            old.available != snapshot.available
        };
        if changed {
            self.imp()
                .gate
                .borrow_mut()
                .owner_changed(snapshot.available);
            self.imp().busy.set(false);
            self.notify_busy();
        }
        let _freeze = self.freeze_notify();
        self.imp().available.set(snapshot.available);
        self.notify_available();
        let output = snapshot
            .sink
            .as_ref()
            .and_then(|name| snapshot.sinks.get(name));
        let microphone = snapshot
            .source
            .as_ref()
            .and_then(|name| snapshot.sources.get(name));
        self.imp().output_available.set(output.is_some());
        self.notify_output_available();
        self.imp().microphone_available.set(microphone.is_some());
        self.notify_microphone_available();
        self.imp()
            .volume
            .set(output.map(|d| d.level.clamp(0.0, 1.0)).unwrap_or(0.0));
        self.notify_volume();
        self.imp().muted.set(output.is_some_and(|d| d.muted));
        self.notify_muted();
        self.imp()
            .microphone_muted
            .set(microphone.is_some_and(|d| d.muted));
        self.notify_microphone_muted();
        self.imp().snapshot.replace(snapshot);
        self.emit_by_name::<()>("changed", &[]);
    }
    pub fn snapshot(&self) -> Snapshot {
        self.imp().snapshot.borrow().clone()
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    pub async fn change(&self, change: Change) -> Result<f64, crate::error::ServiceError> {
        let ticket = self
            .imp()
            .gate
            .borrow_mut()
            .begin()
            .ok_or_else(|| gettext("The audio service is unavailable or busy."))?;
        self.imp().busy.set(true);
        self.notify_busy();
        self.emit_by_name::<()>("changed", &[]);
        let microphone = matches!(change, Change::Microphone);
        let result = async {
            let sender = self
                .imp()
                .worker
                .borrow()
                .clone()
                .ok_or_else(|| gettext("The audio service is unavailable."))?;
            let (reply, response) = async_channel::bounded(1);
            sender
                .send(Work::Request(change, reply))
                .await
                .map_err(|_| gettext("The audio service is unavailable."))?;
            futures_lite::future::race(
                async {
                    response
                        .recv()
                        .await
                        .map_err(|_| gettext("The audio service is unavailable."))?
                },
                async {
                    glib::timeout_future(Duration::from_secs(5)).await;
                    Err(gettext("The audio service did not respond."))
                },
            )
            .await
        }
        .await;
        if !self.imp().gate.borrow_mut().finish(ticket) {
            return Err(
                gettext("The audio device changed while the operation was pending.").into(),
            );
        }
        self.imp().busy.set(false);
        self.notify_busy();
        match result {
            Ok(snapshot) => {
                self.apply(snapshot);
                self.imp().last_error.replace(String::new());
                self.notify_last_error();
                Ok(if microphone {
                    if self.microphone_muted() {
                        0.0
                    } else {
                        1.0
                    }
                } else if self.muted() {
                    0.0
                } else {
                    self.volume()
                })
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
