use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Prefer the journal; keep diagnostics visible in terminals without journald.
pub fn init(app_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if std::env::var_os("ASENA_LOG_STDERR").is_some() {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(std::io::stderr),
            )
            .try_init()?;
        return Ok(());
    }
    match tracing_journald::layer() {
        Ok(journal) => tracing_subscriber::registry()
            .with(filter)
            .with(journal.with_syslog_identifier(app_name.to_owned()))
            .try_init()?,
        Err(_) => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .try_init()?,
    }
    Ok(())
}
