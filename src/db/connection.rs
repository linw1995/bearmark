use diesel::pg::PgConnection;
use dotenvy::dotenv;
use std::env;

pub type Connection = PgConnection;

pub fn establish() -> Connection {
    use diesel::Connection;
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    if cfg!(debug_assertions) {
        use diesel::connection::InstrumentationEvent;
        use tracing::debug;
        conn.set_instrumentation(|event: InstrumentationEvent<'_>| debug!(?event));
    }
    conn
}

pub fn run_migrations() {
    use diesel::connection::{Connection, InstrumentationEvent};
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use tracing::debug;

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
    let mut conn = establish();
    conn.set_instrumentation(|event: InstrumentationEvent<'_>| debug!(?event));
    conn.run_pending_migrations(MIGRATIONS).unwrap();
}
