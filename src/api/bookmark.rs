use crate::db::bookmark::{Bookmark, NewBookmark};
use crate::db::connection;
use crate::db::schema::bookmarks;

use diesel::{RunQueryDsl, SelectableHelper};
use rocket::serde::json::Json;

#[post("/bookmark", format = "application/json", data = "<bookmark>")]
pub fn create_bookmark(bookmark: Json<NewBookmark>) -> Json<Bookmark> {
    let mut conn = connection::establish();

    let m = diesel::insert_into(bookmarks::table)
        .values(&bookmark.into_inner())
        .returning(Bookmark::as_returning())
        .get_result(&mut conn)
        .expect("Error saving new bookmark");

    Json(m)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_bookmark]
}

#[cfg(test)]
mod test {
    use super::*;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn create_bookmark() {
        let app = rocket::build().mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
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
