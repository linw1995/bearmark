pub fn setup_console_log() {
    use std::io;
    use tracing_subscriber::{prelude::*, EnvFilter};

    let console_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stdout)
        .boxed();

    tracing_subscriber::registry()
        .with(vec![console_log])
        .with(EnvFilter::from_default_env())
        .init();
}
