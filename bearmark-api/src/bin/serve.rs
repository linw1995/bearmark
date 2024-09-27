#[macro_use]
extern crate rocket;

use bearmark_api::api::configs::{self, Config};
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

    let cfg_provider = configs::config_provider();
    let ui_path = cfg_provider
        .extract_inner::<Option<String>>("ui_path")
        .unwrap();

    let mut builder = rocket::custom(cfg_provider);
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
