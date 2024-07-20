use super::errors::Error;
use super::fairings::db::Db;
use crate::db::{bookmark, tag};
use crate::utils::{search, BearQLError};

use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_db_pools::Connection;
use tracing::debug;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateBookmarkPayload {
    title: String,
    url: String,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bookmark {
    id: i32,
    title: String,
    url: String,
    tags: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
}

#[post("/", format = "application/json", data = "<payload>")]
pub async fn create_bookmark(
    mut db: Connection<Db>,
    payload: Json<CreateBookmarkPayload>,
) -> Json<Bookmark> {
    let payload = payload.into_inner();
    let (new_bookmark, tags) = (
        bookmark::NewBookmark {
            title: payload.title,
            url: payload.url,
        },
        payload.tags,
    );
    let m = bookmark::create_bookmark(&mut db, new_bookmark).await;
    tag::update_bookmark_tags(&mut db, &m, &tags).await;
    Json(Bookmark {
        id: m.id,
        title: m.title,
        url: m.url,
        tags,
        created_at: m.created_at,
        updated_at: m.updated_at,
        deleted_at: m.deleted_at,
    })
}

#[get("/?<q>&<before>&<limit>")]
pub async fn search_bookmarks(
    mut db: Connection<Db>,
    q: Option<&str>,
    before: Option<i32>,
    limit: Option<i64>,
) -> Result<Json<Vec<Bookmark>>, Error> {
    use pratt_gen::Arena;

    let rv = if let Some(q) = q {
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        let rv = search::parse_query(q, &out_arena, &err_arena)?;
        debug!(?rv, "the query conditions for where clause");

        // check if it has empty keyword
        if rv.keywords.iter().any(|f| f.trim().is_empty()) {
            return Err(BearQLError::EmptyKeyword.into());
        }
        // check if it has empty tag
        if rv.tags.iter().any(|f| f.trim().is_empty()) {
            return Err(BearQLError::EmptyTag.into());
        }

        tag::search_bookmarks(
            &mut db,
            &rv.keywords,
            &rv.tags,
            before.unwrap_or_default(),
            limit.unwrap_or(10),
        )
        .await
    } else {
        tag::search_bookmarks(
            &mut db,
            &vec![],
            &vec![],
            before.unwrap_or_default(),
            limit.unwrap_or(10),
        )
        .await
    };

    debug!(?rv, "search results");

    Ok(Json(
        rv.into_iter()
            .map(|(m, tags)| Bookmark {
                id: m.id,
                title: m.title,
                url: m.url,
                tags: tags.into_iter().map(|t| t.name).collect(),
                created_at: m.created_at,
                updated_at: m.updated_at,
                deleted_at: m.deleted_at,
            })
            .collect(),
    ))
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ModifyBookmark {
    pub title: Option<String>,
    pub url: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[put("/<id>", format = "application/json", data = "<payload>")]
pub async fn update_bookmark(
    mut db: Connection<Db>,
    id: i32,
    payload: Json<ModifyBookmark>,
) -> Result<Json<Bookmark>, Error> {
    let payload = payload.into_inner();
    let (modify_bookmark, modify_tags) = (
        if payload.title.is_some() || payload.url.is_some() {
            Some(bookmark::ModifyBookmark {
                title: payload.title,
                url: payload.url,
            })
        } else {
            None
        },
        payload.tags,
    );
    if modify_bookmark.is_none() && modify_tags.is_none() {
        return Err(Error::BadRequest("No changes".to_string()));
    }

    let m = if let Some(payload) = modify_bookmark {
        bookmark::update_bookmark(&mut db, id, payload).await
    } else {
        bookmark::Bookmark::get(&mut db, id).await
    }
    .ok_or_else(|| Error::NotFound("Bookmark not found".to_string()))?;

    if let Some(payload) = modify_tags {
        tag::update_bookmark_tags(&mut db, &m, &payload).await;
    }

    let rv = tag::get_tags_per_bookmark(&mut db, vec![m.clone()]).await;
    if let Some((m, tags)) = rv.first() {
        return Ok(Json(Bookmark {
            id: m.id,
            title: m.title.clone(),
            url: m.url.clone(),
            tags: tags.iter().map(|t| t.name.clone()).collect(),
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }));
    }

    Ok(Json(Bookmark {
        id: m.id,
        title: m.title,
        url: m.url,
        tags: vec![],
        created_at: m.created_at,
        updated_at: m.updated_at,
        deleted_at: m.deleted_at,
    }))
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
    use crate::utils;

    use super::*;
    use bookmark::tests::rand_bookmark;

    use itertools::Itertools;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket_db_pools::Database;
    use tracing::info;

    fn test_client() -> Client {
        let app = rocket::build().attach(Db::init()).mount("/", routes());
        let client = Client::tracked(app).expect("valid rocket instance");
        return client;
    }

    #[test]
    fn create_bookmark() {
        let client = test_client();
        let payload = CreateBookmarkPayload {
            url: "https://www.rust-lang.org".to_string(),
            title: "Rust".to_string(),
            tags: vec!["rust".to_string(), "programming".to_string()],
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
        let client = test_client();
        let payload = CreateBookmarkPayload {
            url: "https://www.rust-lang.org".to_string(),
            title: "Rust".to_string(),
            tags: vec!["rust".to_string(), "programming".to_string()],
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
        let mut conn = crate::db::connection::establish().await;
        crate::db::tag::tests::setup_searchable_bookmarks(&mut conn).await;

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
            "/?q=Weather",
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            "/?q=Weather&limit=2",
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            format!("/?q=Weather&before={}&limit=2", results[1].id),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            format!("/?q={}", utils::percent_encoding("#global weather")),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            format!("/?q={}", utils::percent_encoding("#west weather")),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            format!("/?q={}", utils::percent_encoding("#global #west weather")),
            results.len() == 0,
            "Expected 0 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            format!("/?q={}", utils::percent_encoding("#weather")),
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );
        assert_get_bookmarks!(
            format!("/?q={}&limit=1", utils::percent_encoding("#weather")),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );
        assert_get_bookmarks!(
            format!(
                "/?q={}&limit=3&before={}",
                utils::percent_encoding("#weather"),
                results[0].id
            ),
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );
    }

    #[test]
    fn unsearchable_deleted_bookmark() {
        let payload = crate::db::bookmark::tests::rand_bookmark();
        let payload = CreateBookmarkPayload {
            url: payload.url,
            title: payload.title,
            tags: vec!["rust".to_string(), "programming".to_string()],
        };
        info!(?payload, "creating");
        let title = payload.title.clone();
        let client = test_client();
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
                let response = client.get(format!("/?q={}", title)).dispatch();
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
        let client = test_client();
        let m = rand_bookmark();
        let payload = CreateBookmarkPayload {
            url: m.url,
            title: m.title,
            tags: vec!["rust".to_string(), "programming".to_string()],
        };
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        let payload = ModifyBookmark {
            url: Some("https://www.rust-lang.org".to_string()),
            title: Some("Rust Programming Language".to_string()),
            tags: None,
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
        let client = test_client();
        let payload = ModifyBookmark {
            url: Some("https://www.rust-lang.org".to_string()),
            title: Some("Rust Programming Language".to_string()),
            tags: None,
        };

        let response = client.put("/99999999").json(&payload).dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    fn update_bookmark_no_change() {
        let client = test_client();
        let m = rand_bookmark();
        let payload = CreateBookmarkPayload {
            url: m.url,
            title: m.title,
            tags: vec!["rust".to_string(), "programming".to_string()],
        };
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        let payload = ModifyBookmark {
            url: None,
            title: None,
            tags: None,
        };
        let response = client
            .put(format!("/{}", added.id))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[test]
    fn update_bookmark_tags() {
        let client = test_client();
        let m = rand_bookmark();
        let payload = CreateBookmarkPayload {
            url: m.url,
            title: m.title,
            tags: vec!["rust", "programming"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        };
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        let modify_tags = vec!["doc", "programming", "rust"]
            .into_iter()
            .map(|s| s.to_string())
            .collect_vec();
        let payload = ModifyBookmark {
            url: None,
            title: None,
            tags: Some(modify_tags.clone()),
        };

        let response = client
            .put(format!("/{}", added.id))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let updated: Bookmark = response.into_json().unwrap();

        assert_eq!(updated.id, added.id);
        assert_eq!(updated.title, added.title);
        assert_eq!(updated.url, added.url);
        assert_eq!(updated.tags, modify_tags);
    }
}
