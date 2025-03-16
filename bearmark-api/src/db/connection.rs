use diesel_async::AsyncPgConnection;

use crate::api::configs;

pub async fn establish() -> AsyncPgConnection {
    use diesel_async::AsyncConnection;

    let url = configs::get_database_url();

    let mut conn = AsyncPgConnection::establish(&url)
        .await
        .unwrap_or_else(|_| panic!("Error connecting database",));

    if cfg!(debug_assertions) {
        use diesel::connection::InstrumentationEvent;
        conn.set_instrumentation(|event: InstrumentationEvent<'_>| match event {
            InstrumentationEvent::StartQuery { query, .. } => {
                tracing::info!("Executing query: {}", query);
            }
            InstrumentationEvent::FinishQuery { query, error, .. } => match error {
                Some(e) => tracing::error!("Query failed: {}\nError: {:?}", query, e),
                None => tracing::debug!("Executing query succeeded: {}", query),
            },
            _ => {}
        });
    }

    conn
}

pub async fn run_migrations() {
    use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
    use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations");
    let conn = establish().await;

    let mut async_wrapper: AsyncConnectionWrapper<AsyncPgConnection> =
        AsyncConnectionWrapper::from(conn);

    _ = tokio::task::spawn_blocking(move || {
        async_wrapper.run_pending_migrations(MIGRATIONS).unwrap();
    })
    .await;
}
