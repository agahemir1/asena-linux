use asena_core::i18n::gettext;

#[derive(Clone, Debug)]
pub enum Action {
    Menu,
    QuickSettings,
    Notifications,
    Osd { kind: String, value: f64 },
    ReloadStyle,
    Audio(asena_services::audio::Change),
    Brightness(f64),
}

pub struct Command {
    pub action: Action,
    pub reply: async_channel::Sender<Result<(), String>>,
}

pub struct ShellBus {
    pub commands: async_channel::Sender<Command>,
}

impl ShellBus {
    async fn dispatch(&self, action: Action) -> zbus::fdo::Result<()> {
        let (reply, response) = async_channel::bounded(1);
        self.commands
            .send(Command { action, reply })
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?;
        response
            .recv()
            .await
            .map_err(|_| zbus::fdo::Error::Failed(gettext("The shell is shutting down.")))?
            .map_err(zbus::fdo::Error::Failed)
    }
}

#[zbus::interface(name = "org.asenalinux.Shell")]
impl ShellBus {
    async fn change_brightness(&self, action: &str) -> zbus::fdo::Result<()> {
        let step = match action {
            "up" => 0.1,
            "down" => -0.1,
            _ => {
                return Err(zbus::fdo::Error::InvalidArgs(gettext(
                    "Invalid brightness action.",
                )))
            }
        };
        self.dispatch(Action::Brightness(step)).await
    }
    async fn change_volume(&self, action: &str) -> zbus::fdo::Result<()> {
        use asena_services::audio::Change;
        let change = match action {
            "up" => Change::Step(0.05),
            "down" => Change::Step(-0.05),
            "mute" => Change::Mute,
            _ => {
                return Err(zbus::fdo::Error::InvalidArgs(gettext(
                    "Invalid audio action.",
                )))
            }
        };
        self.dispatch(Action::Audio(change)).await
    }
    async fn toggle_microphone(&self) -> zbus::fdo::Result<()> {
        self.dispatch(Action::Audio(asena_services::audio::Change::Microphone))
            .await
    }

    async fn toggle_menu(&self) -> zbus::fdo::Result<()> {
        self.dispatch(Action::Menu).await
    }
    async fn toggle_quick_settings(&self) -> zbus::fdo::Result<()> {
        self.dispatch(Action::QuickSettings).await
    }
    async fn toggle_notification_center(&self) -> zbus::fdo::Result<()> {
        self.dispatch(Action::Notifications).await
    }
    async fn show_osd(&self, kind: &str, value: f64) -> zbus::fdo::Result<()> {
        if !["volume", "brightness", "microphone", "keyboard"].contains(&kind)
            || !value.is_finite()
            || !(0.0..=1.0).contains(&value)
        {
            return Err(zbus::fdo::Error::InvalidArgs(gettext(
                "Invalid on-screen display value.",
            )));
        }
        tracing::debug!(kind, value, "OSD requested");
        self.dispatch(Action::Osd {
            kind: kind.into(),
            value,
        })
        .await
    }
    async fn reload_style(&self) -> zbus::fdo::Result<()> {
        self.dispatch(Action::ReloadStyle).await
    }
}

pub async fn connect(commands: async_channel::Sender<Command>) -> zbus::Result<zbus::Connection> {
    let connection = zbus::connection::Builder::session()?
        .serve_at(asena_core::dbus::SHELL_PATH, ShellBus { commands })?
        .build()
        .await?;
    asena_core::dbus::claim_name(&connection, asena_core::dbus::SHELL_ID).await?;
    Ok(connection)
}

pub async fn reload_style() -> Result<(), String> {
    if std::env::var_os("ASENA_DEV").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Err(gettext(
            "Style reload is only available in a development session.",
        ));
    }
    let source = std::env::var_os("ASENA_SOURCE_DIR")
        .ok_or_else(|| gettext("The development source directory is unavailable."))?;
    let path = std::env::var_os("ASENA_DEV_STYLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(source).join("data/style/shell.css"));
    let file = gio::File::for_path(path);
    use gio::prelude::*;
    let (bytes, _) = file
        .load_contents_future()
        .await
        .map_err(|error| format!("{}: {error}", gettext("Could not read the style file.")))?;
    let css = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    asena_widgets::reload_style(css)
}
