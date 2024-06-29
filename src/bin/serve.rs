#[macro_use]
extern crate rocket;

use bmm::api::bookmark;
use bmm::api::fairings::db::Db;

use rocket_db_pools::Database;

#[launch]
#[cfg(not(tarpaulin_include))]
fn rocket() -> _ {
    bmm::utils::logging::setup_console_log();
    bmm::db::connection::run_migrations(); // diesel_async not supports instrumentationEvent. So use diesel instead. Only for running migrations.

    rocket::build()
        .attach(Db::init())
        .mount("/bookmarks", bookmark::routes())
}
