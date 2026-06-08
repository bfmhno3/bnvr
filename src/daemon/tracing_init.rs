use tracing_subscriber::fmt::writer::MakeWriterExt;

pub fn init() {
    let log_dir = super::paths::log_dir();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "bnvr.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr.and(non_blocking))
        .init();

    // Leak the guard so the file appender stays alive for the process lifetime.
    // Flushing happens on process exit or via the non_blocking background thread.
    std::mem::forget(_guard);
}
