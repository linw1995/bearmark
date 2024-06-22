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

#[derive(Responder)]
pub enum Error {
    #[response(status = 404, content_type = "json")]
    NotFound(String),
}

#[delete("/<id>")]
pub fn delete_bookmark(id: i32) -> Result<&'static str, Error> {
    let effected = bookmark::delete_bookmarks(vec![id]) == 1;
    if effected {
        Ok("Deleted")
    } else {
        Err(Error::NotFound("Bookmark not found".to_string()))
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_bookmark, search_bookmarks, delete_bookmark]
}

#[cfg(test)]
mod tests {
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
    fn delete_bookmark() {
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

        let response = client.delete(format!("/{}", added.id)).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.delete(format!("/{}", added.id)).dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    #[serial(search_bookmarks)]
    fn search_bookmarks() {
        // Create some bookmarks
        crate::db::bookmark::tests::search_bookmarks_with_pagination();

        let app = rocket::build().mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
        let mut results: Vec<Bookmark>;

        macro_rules! assert_get_bookmarks {
            ($uri:expr, $($assert_args:expr),*) => {
                let response = client.get($uri).dispatch();
                assert_eq!(response.status(), Status::Ok);
                results = response.into_json().unwrap();
                assert!(
                    $($assert_args,)*
                );
            };
        }

        assert_get_bookmarks!(
            "/",
            results.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            "/?title=Weather",
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            "/?title=Weather&limit=2",
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            format!("/?title=Weather&before={}&limit=2", results[1].id),
            results.len() == 1,
            "Expected 1 bookmarks, got {}",
            results.len()
        );
    }

    #[test]
    #[serial(unsearchable_deleted_bookmark)]
    fn unsearchable_deleted_bookmark() {
        let title = "invisible";
        let app = rocket::build().mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
        let payload = NewBookmark {
            url: "https://www.rust-lang.org".to_string(),
            title: title.to_string(),
        };
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        let mut results: Vec<Bookmark>;

        macro_rules! assert_get_bookmark {
            ($($assert_args:expr),*) => {
                let response = client.get(format!("/?title={}", title)).dispatch();
                assert_eq!(response.status(), Status::Ok);
                results = response.into_json().unwrap();
                assert!(
                    $($assert_args,)*
                );
            };
        }

        assert_get_bookmark!(
            results.len() == 1,
            "Expected 1 bookmarks, got {}",
            results.len()
        );

        let response = client.delete(format!("/{}", added.id)).dispatch();
        assert_eq!(response.status(), Status::Ok);

        assert_get_bookmark!(
            results.len() == 0,
            "Expected 0 bookmarks, got {}",
            results.len()
        );
    }
}
