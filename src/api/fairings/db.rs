use diesel::connection::InstrumentationEvent;
use diesel_async::{
    pooled_connection::{
        deadpool::{BuildError, Object, Pool, PoolError},
        AsyncDieselConnectionManager,
    },
    AsyncConnection, AsyncPgConnection,
};
use dotenvy::dotenv;
use rocket::figment::Figment;
use rocket_db_pools::{Database, Error};

pub type InitError = BuildError;
pub type GetError = PoolError;
pub type Connection = AsyncPgConnection;
pub struct DBPool(Pool<Connection>);

#[rocket::async_trait]
impl rocket_db_pools::Pool for DBPool {
    type Connection = Object<Connection>;

    type Error = Error<InitError, GetError>;

    async fn init(_figment: &Figment) -> Result<Self, Self::Error> {
        dotenv().ok();

        let url = std::env::var("DATABASE_URL").expect("env DATABASE_URL must be set");
        let config = AsyncDieselConnectionManager::<Connection>::new(url);
        match Pool::builder(config).build() {
            Ok(pool) => Ok(Self(pool)),
            Err(e) => Err(Error::Init(e)),
        }
    }

    async fn get(&self) -> Result<Self::Connection, Self::Error> {
        // Get one connection from the pool, here via an `acquire()` method.
        // Map errors of type `GetError` to `Error<_, GetError>`.
        let mut conn = self.0.get().await.map_err(Error::Get)?;

        conn.set_instrumentation(|event: InstrumentationEvent<'_>| match event {
            InstrumentationEvent::StartQuery { query, .. } => {
                tracing::debug!("Executing query: {}", query);
            }
            InstrumentationEvent::FinishQuery { query, error, .. } => match error {
                Some(e) => tracing::error!("Query failed: {}\nError: {:?}", query, e),
                None => tracing::debug!("Executing query succeeded: {}", query),
            },
            _ => {}
        });

        Ok(conn)
    }

    async fn close(&self) {
        self.0.close()
    }
}

#[derive(Database)]
#[database("deadpool_diesel_postgres")]
pub struct Db(DBPool);
