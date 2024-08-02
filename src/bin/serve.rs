#[macro_use]
extern crate rocket;

use bearmark::api::fairings::db::Db;
use bearmark::api::{bookmark, folder, tag};

use rocket_db_pools::Database;

#[launch]
#[cfg(not(tarpaulin_include))]
async fn rocket() -> _ {
    bearmark::utils::logging::setup_console_log();
    bearmark::db::connection::run_migrations().await;

    rocket::build()
        .attach(Db::init())
        .mount("/bookmarks", bookmark::routes())
        .mount("/tags", tag::routes())
        .mount("/folders", folder::routes())
}
