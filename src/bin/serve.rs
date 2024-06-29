#[macro_use]
extern crate rocket;

use bmm::api::bookmark;

#[launch]
#[cfg(not(tarpaulin_include))]
async fn rocket() -> _ {
    bmm::utils::logging::setup_console_log();
    bmm::db::connection::run_migrations().await;

    rocket::build().mount("/bookmarks", bookmark::routes())
}
