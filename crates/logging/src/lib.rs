use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_logger() -> anyhow::Result<()> {
    #[cfg(feature = "file-logging")]
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("app.log")?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("rustls::client::hs=error,trace"));

    // Terminal
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .with_ansi(true);

    // File
    #[cfg(feature = "file-logging")]
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .with_ansi(false);

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer);

    #[cfg(feature = "file-logging")]
    let subscriber = subscriber.with(file_layer);

    subscriber.init();

    Ok(())
}
