use diesel::pg::PgConnection;
use dotenvy::dotenv;
use std::env;

pub type Connection = PgConnection;

pub fn establish() -> Connection {
    use diesel::{connection::InstrumentationEvent, Connection};

    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    if cfg!(debug_assertions) {
        use tracing::debug;
        conn.set_instrumentation(|event: InstrumentationEvent<'_>| debug!(?event));
    }
    conn
}
