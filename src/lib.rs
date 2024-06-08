use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub mod models;
pub mod schema;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use std::io;
    use tracing_subscriber::{filter::LevelFilter, prelude::*};

    let console_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stdout)
        .with_filter(LevelFilter::INFO)
        .boxed();

    tracing_subscriber::registry()
        .with(vec![console_log])
        .init();
}
