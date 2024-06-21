#[macro_use]
extern crate rocket;

use bmm::api::bookmark;

#[launch]
fn rocket() -> _ {
    bmm::utils::logging::setup_console_log();
    bmm::db::connection::run_migrations();

    rocket::build().mount("/bookmarks", bookmark::routes())
}
