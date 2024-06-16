#[macro_use]
extern crate rocket;

pub mod api;
pub mod db;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use std::io;
    use tracing_subscriber::prelude::*;

    let console_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stdout)
        .boxed();

    tracing_subscriber::registry()
        .with(vec![console_log])
        .init();
}
