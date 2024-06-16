#[macro_use]
extern crate rocket;

use bmm::api::bookmark;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", bookmark::routes())
}
