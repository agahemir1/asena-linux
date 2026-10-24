//! The recovery system bus service and the offline reset window (K30).
mod service;
mod window;

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|argument| argument == "--service") {
        asena_core::logging::init("asena-recovery").map_err(|error| anyhow::anyhow!("{error}"))?;
        return service::run();
    }
    window::run();
    Ok(())
}
