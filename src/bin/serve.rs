#[macro_use]
extern crate rocket;

use bearmark::api::fairings::db::Db;
use bearmark::api::{bookmark, folder, tag};

use rocket_db_pools::Database;

#[launch]
#[cfg(not(tarpaulin_include))]
async fn rocket() -> _ {
    use rocket::fs::FileServer;

    bearmark::utils::logging::setup_console_log();
    bearmark::db::connection::run_migrations().await;

    rocket::build()
        .mount("/", FileServer::from("./static"))
        .attach(Db::init())
        .mount("/api/bookmarks", bookmark::routes())
        .mount("/api/tags", tag::routes())
        .mount("/api/folders", folder::routes())
}
