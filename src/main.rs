#[macro_use]
extern crate rocket;

use bmm::models::{Bookmark, NewBookmark};
use diesel::{RunQueryDsl, SelectableHelper};
use rocket::serde::json::Json;

#[post("/bookmark", format = "application/json", data = "<bookmark>")]
fn create_bookmark(bookmark: Json<NewBookmark>) -> String {
    let mut conn = bmm::establish_connection();

    let m = diesel::insert_into(bmm::schema::bookmarks::table)
        .values(&bookmark.into_inner())
        .returning(Bookmark::as_returning())
        .get_result(&mut conn)
        .expect("Error saving new bookmark");

    format!("Bookmark#{} added", m.id)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![create_bookmark])
}
