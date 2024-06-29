use diesel::PgConnection;
use diesel_async::AsyncPgConnection;
use dotenvy::dotenv;
use std::env;

pub async fn establish_async() -> AsyncPgConnection {
    use diesel_async::AsyncConnection;
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    AsyncPgConnection::establish(&database_url)
        .await
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn establish() -> PgConnection {
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

pub async fn run_migrations_async() {
    use diesel::connection::InstrumentationEvent;
    use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use tracing::debug;

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
    let conn = establish_async().await;
    _ = diesel::connection::set_default_instrumentation(|| {
        Some(Box::new(|event: InstrumentationEvent<'_>| debug!(?event)))
    });

    let mut async_wrapper: AsyncConnectionWrapper<AsyncPgConnection> =
        AsyncConnectionWrapper::from(conn);

    _ = tokio::task::spawn_blocking(move || {
        async_wrapper.run_pending_migrations(MIGRATIONS).unwrap();
    })
    .await;

    _ = diesel::connection::set_default_instrumentation(|| None);
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
