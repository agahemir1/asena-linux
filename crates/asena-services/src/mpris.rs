//! MPRIS player registry. Playing players outrank recent paused players.
use crate::property_model::{Bus, Endpoint, PropertyModel};
use asena_core::i18n::gettext;
use futures_lite::StreamExt;
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    sync::OnceLock,
};
use zbus::zvariant::OwnedValue;
const PREFIX: &str = "org.mpris.MediaPlayer2.";
#[derive(Clone)]
struct Player {
    state: PropertyModel,
    identity: PropertyModel,
}
#[derive(Clone, Debug, Default)]
pub struct Metadata {
    pub title: String,
    pub artist: String,
    pub art_url: String,
}
fn metadata(model: &PropertyModel) -> Metadata {
    model.with_values(|values| {
        let data = values
            .get("Metadata")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| std::collections::HashMap::<String, OwnedValue>::try_from(v).ok())
            .unwrap_or_default();
        Metadata {
            title: data
                .get("xesam:title")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .into(),
            artist: data
                .get("xesam:artist")
                .and_then(|v| v.try_clone().ok())
                .and_then(|v| Vec::<String>::try_from(v).ok())
                .unwrap_or_default()
                .join(", "),
            art_url: data
                .get("mpris:artUrl")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .into(),
        }
    })
}
mod imp {
    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::MprisModel)]
    pub struct MprisModel {
        #[property(get)]
        pub available: Cell<bool>,
        #[property(get)]
        pub playing: Cell<bool>,
        #[property(get)]
        pub can_toggle: Cell<bool>,
        #[property(get)]
        pub can_next: Cell<bool>,
        #[property(get)]
        pub can_previous: Cell<bool>,
        #[property(get)]
        pub busy: Cell<bool>,
        #[property(get)]
        pub title: RefCell<String>,
        #[property(get)]
        pub artist: RefCell<String>,
        #[property(get)]
        pub identity: RefCell<String>,
        #[property(get)]
        pub art_url: RefCell<String>,
        #[property(get)]
        pub last_error: RefCell<String>,
        pub(super) players: RefCell<BTreeMap<String, Player>>,
        pub recent: RefCell<BTreeMap<String, u64>>,
        pub counter: Cell<u64>,
        pub selected: RefCell<Option<String>>,
        pub task: RefCell<Option<glib::JoinHandle<()>>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for MprisModel {
        const NAME: &'static str = "AsenaMprisModel";
        type Type = super::MprisModel;
    }
    #[glib::derived_properties]
    impl ObjectImpl for MprisModel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
        fn dispose(&self) {
            if let Some(task) = self.task.borrow_mut().take() {
                task.abort();
            }
            self.players.borrow_mut().clear();
        }
    }
}
glib::wrapper! {pub struct MprisModel(ObjectSubclass<imp::MprisModel>);}
impl Default for MprisModel {
    fn default() -> Self {
        glib::Object::new()
    }
}
impl MprisModel {
    fn attach(&self, name: String) {
        if self.imp().players.borrow().contains_key(&name)
            || self.imp().players.borrow().len() >= 64
        {
            return;
        }
        let endpoint = |interface: &str| Endpoint {
            bus: Bus::Session,
            name: name.clone(),
            path: "/org/mpris/MediaPlayer2".into(),
            interface: interface.into(),
        };
        let player = Player {
            state: PropertyModel::new(endpoint("org.mpris.MediaPlayer2.Player")),
            identity: PropertyModel::new(endpoint("org.mpris.MediaPlayer2")),
        };
        for model in [&player.state, &player.identity] {
            let weak = self.downgrade();
            model.connect_changed(move |_| {
                if let Some(model) = weak.upgrade() {
                    model.refresh();
                }
            });
            model.start();
        }
        self.imp().players.borrow_mut().insert(name, player);
        self.refresh();
    }
    fn remove(&self, name: &str) {
        self.imp().players.borrow_mut().remove(name);
        self.imp().recent.borrow_mut().remove(name);
        self.refresh();
    }
    fn refresh(&self) {
        let players = self.imp().players.borrow();
        let recent = self.imp().recent.borrow();
        let selected = players
            .iter()
            .filter(|(_, player)| player.state.available())
            .max_by_key(|(name, player)| {
                (
                    player.state.string("PlaybackStatus").as_deref() == Some("Playing"),
                    recent.get(*name).copied().unwrap_or(0),
                )
            });
        let _freeze = self.freeze_notify();
        let data = selected
            .map(|(_, player)| metadata(&player.state))
            .unwrap_or_default();
        self.imp()
            .selected
            .replace(selected.map(|(name, _)| name.clone()));
        self.imp().available.set(selected.is_some());
        self.notify_available();
        let playing = selected.is_some_and(|(_, player)| {
            player.state.string("PlaybackStatus").as_deref() == Some("Playing")
        });
        self.imp().playing.set(playing);
        self.notify_playing();
        let can_control =
            selected.is_some_and(|(_, player)| player.state.boolean("CanControl") == Some(true));
        let busy = selected.is_some_and(|(_, player)| player.state.busy());
        self.imp().busy.set(busy);
        self.notify_busy();
        self.imp().can_toggle.set(
            can_control
                && selected.is_some_and(|(_, player)| {
                    player
                        .state
                        .boolean(if playing { "CanPause" } else { "CanPlay" })
                        == Some(true)
                }),
        );
        self.notify_can_toggle();
        self.imp().can_next.set(
            can_control
                && selected
                    .is_some_and(|(_, player)| player.state.boolean("CanGoNext") == Some(true)),
        );
        self.notify_can_next();
        self.imp().can_previous.set(
            can_control
                && selected
                    .is_some_and(|(_, player)| player.state.boolean("CanGoPrevious") == Some(true)),
        );
        self.notify_can_previous();
        self.imp().title.replace(data.title);
        self.notify_title();
        self.imp().artist.replace(data.artist);
        self.notify_artist();
        self.imp().art_url.replace(data.art_url);
        self.notify_art_url();
        self.imp().identity.replace(
            selected
                .and_then(|(_, player)| player.identity.string("Identity"))
                .unwrap_or_default(),
        );
        self.notify_identity();
        self.emit_by_name::<()>("changed", &[]);
    }
    pub fn start(&self) {
        if self.imp().task.borrow().is_some() {
            return;
        }
        let weak = self.downgrade();
        self.imp()
            .task
            .replace(Some(glib::spawn_future_local(async move {
                let result: zbus::Result<()> = async {
                    let connection = zbus::Connection::session().await?;
                    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
                    let mut changes = bus.receive_name_owner_changed().await?;
                    for name in bus.list_names().await? {
                        if name.as_str().starts_with(PREFIX) {
                            if let Some(model) = weak.upgrade() {
                                model.attach(name.to_string());
                            }
                        }
                    }
                    while let Some(signal) = changes.next().await {
                        let args = signal.args()?;
                        let name = args.name().as_str();
                        if !name.starts_with(PREFIX) {
                            continue;
                        }
                        let Some(model) = weak.upgrade() else {
                            return Ok(());
                        };
                        model.remove(name);
                        if args.new_owner().is_some() {
                            model.attach(name.into());
                        }
                    }
                    Ok(())
                }
                .await;
                if let Some(model) = weak.upgrade() {
                    if let Err(error) = result {
                        model.imp().last_error.replace(error.to_string());
                        model.notify_last_error();
                    }
                    model.imp().players.borrow_mut().clear();
                    model.refresh();
                }
            })));
    }
    pub fn connect_changed(&self, callback: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |values| {
            if let Ok(model) = values[0].get::<Self>() {
                callback(&model);
            }
            None
        })
    }
    pub async fn action(&self, method: &str) -> Result<(), crate::error::ServiceError> {
        let allowed = match method {
            "PlayPause" => self.can_toggle(),
            "Next" => self.can_next(),
            "Previous" => self.can_previous(),
            _ => false,
        };
        if !allowed {
            return Err(gettext("The player does not support this action.").into());
        }
        let selected = self
            .imp()
            .selected
            .borrow()
            .clone()
            .ok_or_else(|| gettext("No media player is available."))?;
        let player = self
            .imp()
            .players
            .borrow()
            .get(&selected)
            .cloned()
            .ok_or_else(|| gettext("No media player is available."))?;
        player.state.call_unit(method).await?;
        let counter = self.imp().counter.get().wrapping_add(1);
        self.imp().counter.set(counter);
        self.imp().recent.borrow_mut().insert(selected, counter);
        self.refresh();
        Ok(())
    }
}
