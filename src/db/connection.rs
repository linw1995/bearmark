use diesel::pg::PgConnection;
use dotenvy::dotenv;
use std::env;

pub type Connection = PgConnection;

pub fn establish() -> Connection {
    use diesel::Connection;

    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    Connection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
