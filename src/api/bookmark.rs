use crate::db::bookmark::{self, Bookmark, NewBookmark};
use crate::db::connection;
use crate::db::schema::bookmarks;

use diesel::{RunQueryDsl, SelectableHelper};
use rocket::serde::json::Json;

#[post("/", format = "application/json", data = "<bookmark>")]
pub fn create_bookmark(bookmark: Json<NewBookmark>) -> Json<Bookmark> {
    let mut conn = connection::establish();

    let m = diesel::insert_into(bookmarks::table)
        .values(&bookmark.into_inner())
        .returning(Bookmark::as_returning())
        .get_result(&mut conn)
        .expect("Error saving new bookmark");

    Json(m)
}

#[get("/?<title>&<before>&<limit>")]
pub fn search_bookmarks(
    title: Option<&str>,
    before: Option<i32>,
    limit: Option<i64>,
) -> Json<Vec<Bookmark>> {
    Json(bookmark::search_bookmarks(
        title.unwrap_or_default(),
        before.unwrap_or_default(),
        limit.unwrap_or(10),
    ))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_bookmark, search_bookmarks]
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

    #[test]
    #[serial(search_bookmarks)]
    fn search_bookmarks() {
        // Create some bookmarks
        crate::db::bookmark::tests::search_bookmarks_with_pagination();

        let app = rocket::build().mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");

        let response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let results: Vec<Bookmark> = response.into_json().unwrap();
        assert!(
            results.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            results.len()
        );

        let response = client.get("/?title=Weather").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let results: Vec<Bookmark> = response.into_json().unwrap();
        assert!(
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        let response = client.get("/?title=Weather&limit=2").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let results: Vec<Bookmark> = response.into_json().unwrap();
        assert!(
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        let response = client
            .get(format!("/?title=Weather&before={}&limit=2", results[1].id))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let results: Vec<Bookmark> = response.into_json().unwrap();
        assert!(
            results.len() == 1,
            "Expected 1 bookmarks, got {}",
            results.len()
        );
    }
}
