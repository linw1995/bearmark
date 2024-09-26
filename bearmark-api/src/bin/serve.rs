#[macro_use]
extern crate rocket;

use bearmark_api::api::configs::Config;
use bearmark_api::api::fairings::db::Db;
use bearmark_api::api::{bookmark, folder, tag};

use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use rocket_db_pools::Database;

#[launch]
#[cfg(not(tarpaulin_include))]
async fn rocket() -> _ {
    bearmark_api::utils::logging::setup_console_log();
    bearmark_api::db::connection::run_migrations().await;

    rocket::build()
        .mount("/", FileServer::from("./static"))
        .attach(Db::init())
        .mount("/api/bookmarks", bookmark::routes())
        .mount("/api/tags", tag::routes())
        .mount("/api/folders", folder::routes())
        .attach(AdHoc::config::<Config>())
}
