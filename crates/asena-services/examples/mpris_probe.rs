//! Private-bus player selection, capabilities, actions and disappearing players.
use asena_services::mpris::MprisModel;
use std::{cell::Cell, collections::HashMap, rc::Rc, time::Duration};
use zbus::zvariant::{OwnedValue, Str};
struct Root;
#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    #[zbus(property)]
    fn identity(&self) -> &str {
        "Asena test player"
    }
}
struct Player {
    playing: bool,
    title: String,
}
#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    async fn play_pause(
        &mut self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.playing = !self.playing;
        self.playback_status_changed(&emitter).await?;
        Ok(())
    }
    #[zbus(property)]
    fn playback_status(&self) -> &str {
        if self.playing {
            "Playing"
        } else {
            "Paused"
        }
    }
    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        HashMap::from([(
            "xesam:title".into(),
            OwnedValue::from(Str::from(self.title.clone())),
        )])
    }
}
async fn server(name: &str, title: &str, playing: bool) -> zbus::Result<zbus::Connection> {
    let connection = zbus::connection::Builder::session()?
        .serve_at("/org/mpris/MediaPlayer2", Root)?
        .serve_at(
            "/org/mpris/MediaPlayer2",
            Player {
                playing,
                title: title.into(),
            },
        )?
        .build()
        .await?;
    asena_core::dbus::claim_name(&connection, name).await?;
    Ok(connection)
}
async fn until(condition: impl Fn() -> bool) -> anyhow::Result<()> {
    for _ in 0..150 {
        if condition() {
            return Ok(());
        }
        glib::timeout_future(Duration::from_millis(20)).await;
    }
    anyhow::bail!("MPRIS model condition timed out")
}
async fn probe() -> anyhow::Result<()> {
    let model = MprisModel::default();
    model.start();
    let first = server("org.mpris.MediaPlayer2.AsenaProbeA", "Playing track", true).await?;
    let second = server("org.mpris.MediaPlayer2.AsenaProbeB", "Paused track", false).await?;
    until(|| model.available() && model.title() == "Playing track" && model.playing()).await?;
    anyhow::ensure!(!model.can_next(), "Unsupported next action enabled");
    anyhow::ensure!(
        model.action("Next").await.is_err(),
        "Unsupported action accepted"
    );
    model.action("PlayPause").await?;
    until(|| model.title() == "Playing track" && !model.playing()).await?;
    first
        .release_name("org.mpris.MediaPlayer2.AsenaProbeA")
        .await?;
    until(|| model.title() == "Paused track").await?;
    second
        .release_name("org.mpris.MediaPlayer2.AsenaProbeB")
        .await?;
    until(|| !model.available()).await?;
    println!(
        "MPRIS playing/recent selection, capability gating, PlayPause and player removal passed."
    );
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let loop_ = glib::MainLoop::new(None, false);
    let finished = loop_.clone();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    glib::spawn_future_local(async move {
        let outcome = futures_lite::future::race(probe(), async {
            glib::timeout_future(Duration::from_secs(15)).await;
            Err(anyhow::anyhow!("MPRIS probe timed out"))
        })
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error:#}"),
        };
        finished.quit();
    });
    loop_.run();
    anyhow::ensure!(success.get(), "MPRIS probe failed");
    Ok(())
}
