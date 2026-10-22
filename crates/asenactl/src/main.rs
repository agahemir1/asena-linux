use anyhow::Context;
use asena_core::i18n::gettext;
use clap::{Parser, Subcommand, ValueEnum};
use std::{cell::Cell, rc::Rc};

#[derive(Parser)]
#[command(name = "asenactl", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Menu { action: Toggle },
    QuickSettings { action: Toggle },
    Notifications { action: Toggle },
    ReloadStyle,
    Volume { action: Level },
    Mic { action: Mute },
    Brightness { action: Step },
    Screenshot { mode: Capture },
    Lock,
    Idle { action: IdleAction },
    Suspend,
}
#[derive(Clone, ValueEnum)]
enum IdleAction {
    Dim,
    Restore,
    Lock,
    Off,
    On,
    Suspend,
}
impl IdleAction {
    fn name(&self) -> &'static str {
        match self {
            Self::Dim => "dim",
            Self::Restore => "restore",
            Self::Lock => "lock",
            Self::Off => "off",
            Self::On => "on",
            Self::Suspend => "suspend",
        }
    }
}
#[derive(Clone, ValueEnum)]
enum Toggle {
    Toggle,
}
#[derive(Clone, ValueEnum)]
enum Level {
    Up,
    Down,
    Mute,
}
#[derive(Clone, ValueEnum)]
enum Mute {
    Mute,
}
#[derive(Clone, ValueEnum)]
enum Step {
    Up,
    Down,
}
#[derive(Clone, ValueEnum)]
enum Capture {
    Area,
    Window,
    Screen,
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let connection = zbus::Connection::session()
        .await
        .context(gettext("Could not connect to the session bus."))?;
    if let Command::Screenshot { mode } = &cli.command {
        return screenshot(&connection, !matches!(mode, Capture::Screen)).await;
    }
    if matches!(
        cli.command,
        Command::Lock | Command::Idle { .. } | Command::Suspend
    ) {
        let scope = asena_core::session::current_scope().map_err(|_| {
            anyhow::anyhow!(gettext("The current session could not be identified."))
        })?;
        let proxy = asena_core::dbus::DaemonProxy::new(&connection).await?;
        return match cli.command {
            Command::Lock => proxy
                .lock(&scope)
                .await
                .context(gettext("The session could not be locked.")),
            Command::Idle { action } => proxy
                .idle(&scope, action.name())
                .await
                .context(gettext("The power request failed.")),
            Command::Suspend => proxy
                .idle(&scope, "suspend")
                .await
                .context(gettext("The power request failed.")),
            _ => Ok(()),
        };
    }
    let proxy = asena_core::dbus::ShellProxy::new(&connection).await?;
    let result = match cli.command {
        Command::Brightness { action } => {
            proxy
                .change_brightness(match action {
                    Step::Up => "up",
                    Step::Down => "down",
                })
                .await
        }
        Command::Volume { action } => {
            proxy
                .change_volume(match action {
                    Level::Up => "up",
                    Level::Down => "down",
                    Level::Mute => "mute",
                })
                .await
        }
        Command::Mic { .. } => proxy.toggle_microphone().await,
        Command::Menu { .. } => proxy.toggle_menu().await,
        Command::QuickSettings { .. } => proxy.toggle_quick_settings().await,
        Command::Notifications { .. } => proxy.toggle_notification_center().await,
        Command::ReloadStyle => proxy.reload_style().await,
        _ => return Ok(()),
    };
    result.context(gettext(
        "The shell did not accept the command. Check that Asena is running.",
    ))
}

/// Captures through the portal, stores the image and announces where it went.
async fn screenshot(connection: &zbus::Connection, interactive: bool) -> anyhow::Result<()> {
    use asena_core::screenshot;
    let uri = screenshot::capture(connection, interactive).await?;
    let stamp = screenshot::stamp();
    let path = screenshot::store(&uri, &stamp)?;
    println!("{}", path.display());
    let proxy = zbus::Proxy::new(
        connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await?;
    let hints: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    let summary = gettext("Screenshot saved");
    let body = path.display().to_string();
    let actions: Vec<&str> = Vec::new();
    // A missing notification server must not turn a saved capture into a failure.
    let _: zbus::Result<u32> = proxy
        .call(
            "Notify",
            &(
                "asenactl",
                0_u32,
                "camera-photo-symbolic",
                summary.as_str(),
                body.as_str(),
                actions,
                hints,
                5000_i32,
            ),
        )
        .await;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asenactl").map_err(|e| anyhow::anyhow!("{e}"))?;
    asena_core::i18n::init().map_err(|e| anyhow::anyhow!("{e}"))?;
    let cli = Cli::parse();
    let loop_ = glib::MainLoop::new(None, false);
    let result = Rc::new(Cell::new(false));
    let task_result = result.clone();
    let task_loop = loop_.clone();
    glib::spawn_future_local(async move {
        match run(cli).await {
            Ok(()) => task_result.set(true),
            Err(error) => eprintln!("{error:#}"),
        }
        task_loop.quit();
    });
    loop_.run();
    if !result.get() {
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_grammar_accepts_documented_commands_and_rejects_unknown_actions() {
        for args in [
            vec!["asenactl", "menu", "toggle"],
            vec!["asenactl", "volume", "mute"],
            vec!["asenactl", "screenshot", "window"],
            vec!["asenactl", "reload-style"],
        ] {
            assert!(Cli::try_parse_from(args).is_ok());
        }
        assert!(Cli::try_parse_from(["asenactl", "menu", "open"]).is_err());
        assert!(Cli::try_parse_from(["asenactl", "brightness", "mute"]).is_err());
    }
}
