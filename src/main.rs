#[macro_use]
extern crate rocket;

use bmm::models::{Bookmark, NewBookmark};
use diesel::{RunQueryDsl, SelectableHelper};
use rocket::serde::json::Json;

#[post("/bookmark", format = "application/json", data = "<bookmark>")]
fn create_bookmark(bookmark: Json<NewBookmark>) -> Json<Bookmark> {
    let mut conn = bmm::establish_connection();

    let m = diesel::insert_into(bmm::schema::bookmarks::table)
        .values(&bookmark.into_inner())
        .returning(Bookmark::as_returning())
        .get_result(&mut conn)
        .expect("Error saving new bookmark");

    Json(m)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![create_bookmark])
}

#[cfg(test)]
mod test {
    use super::rocket;
    use bmm::models::{Bookmark, NewBookmark};
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn create_bookmark() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let payload = NewBookmark {
            url: "https://www.rust-lang.org".to_string(),
            title: "Rust".to_string(),
        };
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        assert!(added.id > 0);
        assert_eq!(added.title, payload.title);
        assert_eq!(added.url, payload.url);
    }
}
