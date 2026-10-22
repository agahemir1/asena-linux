//! Run against the VM's disposable null sink/source, never real user hardware.
use asena_services::audio::{AudioModel, Change};
use std::{cell::Cell, rc::Rc, time::Duration};
async fn until(condition: impl Fn() -> bool) -> anyhow::Result<()> {
    for _ in 0..250 {
        if condition() {
            return Ok(());
        }
        glib::timeout_future(Duration::from_millis(20)).await;
    }
    anyhow::bail!("Audio model condition timed out")
}
async fn probe() -> anyhow::Result<()> {
    let model = AudioModel::default();
    model.start()?;
    if std::env::args().any(|arg| arg == "--reconnect") {
        until(|| model.available()).await?;
        let path = std::env::var_os("ASENA_AUDIO_READY_FILE")
            .ok_or_else(|| anyhow::anyhow!("Ready marker is required"))?;
        let ready = path.clone();
        gio::spawn_blocking(move || std::fs::write(ready, "ready"))
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))??;
        until(|| !model.available()).await?;
        gio::spawn_blocking(move || std::fs::write(path, "lost"))
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))??;
        until(|| model.available()).await?;
        println!("Audio server loss and socket-driven reconnection passed.");
        return Ok(());
    }
    until(|| model.available() && model.output_available()).await?;
    let snapshot = model.snapshot();
    anyhow::ensure!(
        snapshot.sink.as_deref() == Some("asena_test"),
        "Use the disposable asena_test sink for this probe"
    );
    let level = model.change(Change::Level(0.3)).await?;
    anyhow::ensure!((level - 0.3).abs() < 0.005, "Wrong applied volume {level}");
    let level = model.change(Change::Step(0.05)).await?;
    anyhow::ensure!((level - 0.35).abs() < 0.005, "Wrong volume step {level}");
    model.change(Change::Mute).await?;
    anyhow::ensure!(model.muted(), "Mute was not confirmed");
    model.change(Change::Mute).await?;
    anyhow::ensure!(!model.muted(), "Unmute was not confirmed");
    if model.microphone_available() {
        model.change(Change::Microphone).await?;
        anyhow::ensure!(
            model.microphone_muted(),
            "Microphone mute was not confirmed"
        );
        model.change(Change::Microphone).await?;
    }
    anyhow::ensure!(
        model.change(Change::Level(f64::NAN)).await.is_err(),
        "Non-finite volume accepted"
    );
    anyhow::ensure!(!model.busy(), "Audio operation remained busy");
    println!("Native audio null sink: confirmed volume, 5% step, mute/unmute, microphone and invalid input passed.");
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let loop_ = glib::MainLoop::new(None, false);
    let finished = loop_.clone();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    glib::spawn_future_local(async move {
        let outcome = futures_lite::future::race(probe(), async {
            glib::timeout_future(Duration::from_secs(30)).await;
            Err(anyhow::anyhow!("Audio probe timed out"))
        })
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error:#}"),
        };
        finished.quit();
    });
    loop_.run();
    anyhow::ensure!(success.get(), "Audio probe failed");
    Ok(())
}
