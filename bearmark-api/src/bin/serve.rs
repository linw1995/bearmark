#[macro_use]
extern crate rocket;

use bearmark_api::api::configs::Config;
use bearmark_api::api::fairings::db::Db;
use bearmark_api::api::{bookmark, folder, tag};

use rocket::fairing::AdHoc;
use rocket::figment::providers::Serialized;
use rocket::fs::FileServer;
use rocket_db_pools::Database;

#[launch]
#[cfg(not(tarpaulin_include))]
async fn rocket() -> _ {
    bearmark_api::utils::logging::setup_console_log();
    bearmark_api::db::connection::run_migrations().await;

    let config = rocket::Config::figment().merge(Serialized::defaults(Config::default()));
    let ui_path = config.extract_inner::<Option<String>>("ui_path").unwrap();

    let mut builder = rocket::build();
    if let Some(ui_path) = ui_path {
        // Serve the UI files if the path is provided
        builder = builder.mount("/", FileServer::from(ui_path));
    }
    builder
        .attach(Db::init())
        .mount("/api/bookmarks", bookmark::routes())
        .mount("/api/tags", tag::routes())
        .mount("/api/folders", folder::routes())
        .attach(AdHoc::config::<Config>())
}
