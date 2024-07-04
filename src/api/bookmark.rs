use super::fairings::db::Db;
use crate::db::bookmark::{self, Bookmark, ModifyBookmark, NewBookmark};

use rocket::serde::json::Json;
use rocket_db_pools::Connection;

#[post("/", format = "application/json", data = "<payload>")]
pub async fn create_bookmark(mut db: Connection<Db>, payload: Json<NewBookmark>) -> Json<Bookmark> {
    let m = bookmark::create_bookmark(&mut db, payload.into_inner()).await;
    Json(m)
}

#[get("/?<title>&<before>&<limit>")]
pub async fn search_bookmarks(
    mut db: Connection<Db>,
    title: Option<&str>,
    before: Option<i32>,
    limit: Option<i64>,
) -> Json<Vec<Bookmark>> {
    Json(
        bookmark::search_bookmarks(
            &mut db,
            title.unwrap_or_default(),
            before.unwrap_or_default(),
            limit.unwrap_or(10),
        )
        .await,
    )
}

#[derive(Responder)]
pub enum Error {
    #[response(status = 404)]
    NotFound(String),
}

#[delete("/<id>")]
pub async fn delete_bookmark(mut db: Connection<Db>, id: i32) -> Result<&'static str, Error> {
    let effected = bookmark::delete_bookmarks(&mut db, vec![id]).await == 1;
    if effected {
        Ok("Deleted")
    } else {
        Err(Error::NotFound("Bookmark not found".to_string()))
    }
}

#[put("/<id>", format = "application/json", data = "<payload>")]
pub async fn update_bookmark(
    mut db: Connection<Db>,
    id: i32,
    payload: Json<ModifyBookmark>,
) -> Result<Json<Bookmark>, Error> {
    bookmark::update_bookmark(&mut db, id, payload.into_inner())
        .await
        .map(Json)
        .ok_or_else(|| Error::NotFound("Bookmark not found".to_string()))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        create_bookmark,
        search_bookmarks,
        delete_bookmark,
        update_bookmark
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    use bookmark::tests::rand_bookmark;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket_db_pools::Database;
    use tracing::info;

    #[test]
    fn create_bookmark() {
        let app = rocket::build().attach(Db::init()).mount("/", routes());
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
        let app = rocket::build().attach(Db::init()).mount("/", routes());
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

    #[rocket::async_test]
    #[file_serial] // For reusing another test setup.
    async fn search_bookmarks() {
        use rocket::local::asynchronous::Client;

        // Create some bookmarks
        let mut conn = crate::db::connection::establish_async().await;
        crate::db::bookmark::tests::setup_searchable_bookmarks(&mut conn).await;

        let app = rocket::build().attach(Db::init()).mount("/", routes());
        let client = Client::tracked(app).await.expect("valid rocket instance");
        let mut results: Vec<Bookmark>;

        macro_rules! assert_get_bookmarks {
            ($uri:expr, $($assert_args:expr),*) => {
                let response = client.get($uri).dispatch().await;
                assert_eq!(response.status(), Status::Ok);
                results = response.into_json().await.unwrap();
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
    fn unsearchable_deleted_bookmark() {
        let payload = crate::db::bookmark::tests::rand_bookmark();
        info!(?payload, "creating");
        let title = payload.title.clone();
        let app = rocket::build().attach(Db::init()).mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();
        info!(?added, "created");

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

    #[test]
    fn update_exist_bookmark() {
        let app = rocket::build().attach(Db::init()).mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
        let payload = rand_bookmark();
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        let payload = ModifyBookmark {
            url: Some("https://www.rust-lang.org".to_string()),
            title: Some("Rust Programming Language".to_string()),
        };
        assert_ne!(Some(added.title), payload.title);
        assert_ne!(Some(added.url), payload.url);

        let response = client
            .put(format!("/{}", added.id))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let updated: Bookmark = response.into_json().unwrap();

        assert_eq!(updated.id, added.id);
        assert_eq!(updated.title, payload.title.unwrap());
        assert_eq!(updated.url, payload.url.unwrap());
    }

    #[test]
    fn update_missing_bookmark() {
        let app = rocket::build().attach(Db::init()).mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
        let payload = ModifyBookmark {
            url: Some("https://www.rust-lang.org".to_string()),
            title: Some("Rust Programming Language".to_string()),
        };

        let response = client.put("/99999999").json(&payload).dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}
