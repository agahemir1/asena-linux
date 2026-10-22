//! Disposable VM fixture: keep one D-Bus owner alive while authentication is held.
use std::{cell::Cell, path::PathBuf, rc::Rc, time::Duration};
fn main() -> anyhow::Result<()> {
    anyhow::ensure!(
        std::env::var("ASENA_LOCK_TEST_VM").as_deref() == Ok("1"),
        "VM fixture flag required"
    );
    let marker = PathBuf::from(
        std::env::var_os("ASENA_LEASE_MARKER")
            .ok_or_else(|| anyhow::anyhow!("Marker is required"))?,
    );
    let abandon = std::env::args().any(|arg| arg == "--abandon");
    let program = std::env::current_dir()?.join("target/debug/asena-lock");
    let main_loop = glib::MainLoop::new(None, false);
    let finished = main_loop.clone();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    glib::spawn_future_local(async move {
        let outcome: Result<(), Box<dyn std::error::Error>> = async {
            let lease = asena_services::lock_client::request_for_sleep(program).await?;
            lease.ready().await?;
            let ready = marker.clone();
            gio::spawn_blocking(move || std::fs::write(ready, "held"))
                .await
                .map_err(|_| "Marker worker failed")??;
            if abandon {
                std::process::exit(0);
            }
            glib::timeout_future(Duration::from_secs(6)).await;
            lease.release().await;
            gio::spawn_blocking(move || std::fs::write(marker, "released"))
                .await
                .map_err(|_| "Marker worker failed")??;
            Ok(())
        }
        .await;
        match outcome {
            Ok(()) => result.set(true),
            Err(error) => eprintln!("{error}"),
        };
        finished.quit();
    });
    main_loop.run();
    anyhow::ensure!(success.get(), "Lock lease probe failed");
    Ok(())
}
