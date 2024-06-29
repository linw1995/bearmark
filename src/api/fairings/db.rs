use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use dotenvy::dotenv;
use rocket::figment::Figment;
use rocket_db_pools::{Database, Error, Pool};

pub type InitError = diesel_async::pooled_connection::deadpool::BuildError;
pub type GetError = diesel_async::pooled_connection::deadpool::PoolError;
pub type Connection = diesel_async::AsyncPgConnection;
pub struct DBPool(diesel_async::pooled_connection::deadpool::Pool<Connection>);

#[rocket::async_trait]
impl Pool for DBPool {
    type Connection = diesel_async::pooled_connection::deadpool::Object<Connection>;

    type Error = Error<InitError, GetError>;

    async fn init(_figment: &Figment) -> Result<Self, Self::Error> {
        dotenv().ok();

        let url = std::env::var("DATABASE_URL").expect("env DATABASE_URL must be set");
        let config = AsyncDieselConnectionManager::<Connection>::new(url);
        match diesel_async::pooled_connection::deadpool::Pool::builder(config).build() {
            Ok(pool) => Ok(Self(pool)),
            Err(e) => Err(Error::Init(e)),
        }
    }

    async fn get(&self) -> Result<Self::Connection, Self::Error> {
        // Get one connection from the pool, here via an `acquire()` method.
        // Map errors of type `GetError` to `Error<_, GetError>`.
        self.0.get().await.map_err(Error::Get)
    }

    async fn close(&self) {
        self.0.close()
    }
}

#[derive(Database)]
#[database("diesel_mysql")]
pub struct Db(DBPool);
