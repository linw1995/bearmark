use super::errors::Error;
use super::fairings::db::Db;
use super::guards;
use crate::db::{self, bookmark, folder, tag};

use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_db_pools::Connection;
use tracing::debug;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct CreateBookmark {
    pub title: String,
    pub url: String,
    pub folder_id: Option<i32>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct Bookmark {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    #[schema(format = DateTime, value_type=String)]
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[schema(format = DateTime, value_type=String, nullable)]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<time::OffsetDateTime>,
    #[schema(format = DateTime, value_type=String)]
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
}

/// Create a new bookmark
#[utoipa::path(
    post,
    path = "/",
    request_body = CreateBookmark,
    responses(
        (status = 200, description = "Bookmark created success", body = Bookmark)
    )
)]
#[post("/", format = "application/json", data = "<payload>")]
pub async fn create_bookmark(
    mut db: Connection<Db>,
    _required: guards::Auth,
    payload: Json<CreateBookmark>,
) -> Result<Json<Bookmark>, Error> {
    let payload = payload.into_inner();
    let (new, tags) = (
        bookmark::NewBookmark {
            title: payload.title,
            url: payload.url,
        },
        payload.tags,
    );
    let (m, f, ts) = db
        .transaction::<_, Error, _>(|db| {
            async move {
                let m = bookmark::create_bookmark(db, &new).await;

                tag::update_bookmark_tags(db, &m, &tags).await;

                if let Some(folder_id) = payload.folder_id {
                    folder::move_bookmarks(db, folder_id, &vec![m.id]).await?;
                }

                Ok(db::get_bookmark_details(db, vec![m]).await.remove(0))
            }
            .scope_boxed()
        })
        .await?;

    Ok(Json(Bookmark {
        id: m.id,
        title: m.title,
        url: m.url,
        folder: f.map(|f| f.path),
        tags: ts.into_iter().map(|t| t.name).collect(),
        created_at: m.created_at,
        updated_at: m.updated_at,
        deleted_at: m.deleted_at,
    }))
}

/// Search bookmarks
#[utoipa::path(
    get,
    path = "/",
    params(
        ("q" = inline(Option<&str>), Query, description = "Search query language"),
        ("cwd" = inline(Option<&str>), Query, description = "The path of folder to search in"),
        ("before" = inline(Option<i32>), Query, description = "The bookmark id to search before"),
        ("limit" = inline(Option<i64>), Query, description = "The limit of search results")
    ),
    responses(
        (status = 200, description = "Bookmarks searched success", body = Vec<Bookmark>)
    )
)]
#[get("/?<q>&<cwd>&<before>&<limit>")]
pub async fn search_bookmarks(
    mut db: Connection<Db>,
    _required: guards::Auth,
    q: Option<&str>,
    cwd: Option<&str>,
    before: Option<i32>,
    limit: Option<i64>,
) -> Result<Json<Vec<Bookmark>>, Error> {
    let rv = crate::db::search_bookmarks(
        &mut db,
        q,
        cwd,
        before.unwrap_or_default(),
        limit.unwrap_or(10),
    )
    .await?;
    debug!(?rv, "search results");

    Ok(Json(
        rv.into_iter()
            .map(|(m, folder, tags)| Bookmark {
                id: m.id,
                title: m.title,
                url: m.url,
                folder: folder.map(|f| f.path),
                tags: tags.into_iter().map(|t| t.name).collect(),
                created_at: m.created_at,
                updated_at: m.updated_at,
                deleted_at: m.deleted_at,
            })
            .collect(),
    ))
}

/// Delete a bookmark
#[utoipa::path(
    delete,
    path = "/{id}",
    params(
        ("id" = inline(Option<i32>), Path, description = "The bookmark id to be deleted")
    ),
    responses(
        (status = 200, description = "Bookmark deleted success")
    )
)]
#[delete("/<id>")]
pub async fn delete_bookmark(
    mut db: Connection<Db>,
    _required: guards::Auth,
    id: i32,
) -> Result<&'static str, Error> {
    let effected = bookmark::delete_bookmarks(&mut db, vec![id]).await == 1;
    if effected {
        Ok("Deleted")
    } else {
        Err(Error::NotFound("Bookmark not found".to_string()))
    }
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ModifyBookmark {
    pub title: Option<String>,
    pub url: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Update a bookmark
#[utoipa::path(
    patch,
    path = "/{id}",
    params(
        ("id" = inline(Option<i32>), Path, description = "The bookmark id to be updated")
    ),
    request_body = ModifyBookmark,
    responses(
        (status = 200, description = "Bookmark updated success", body = Bookmark)
    )
)]
#[patch("/<id>", format = "application/json", data = "<payload>")]
pub async fn update_bookmark(
    mut db: Connection<Db>,
    _required: guards::Auth,
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

    let (m, folder, tags) = db
        .transaction::<_, Error, _>(|db| {
            async move {
                let m = if let Some(payload) = modify_bookmark {
                    bookmark::update_bookmark(db, id, payload).await
                } else {
                    bookmark::Bookmark::get(db, id).await
                }
                .ok_or_else(|| Error::NotFound("Bookmark not found".to_string()))?;

                if let Some(payload) = modify_tags {
                    tag::update_bookmark_tags(db, &m, &payload).await;
                }

                Ok(db::get_bookmark_details(db, vec![m.clone()])
                    .await
                    .remove(0))
            }
            .scope_boxed()
        })
        .await?;

    Ok(Json(Bookmark {
        id: m.id,
        title: m.title.clone(),
        url: m.url.clone(),
        folder: folder.clone().map(|f| f.path),
        tags: tags.iter().map(|t| t.name.clone()).collect(),
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

pub(crate) mod misc {
    use super::*;

    use utoipa::{OpenApi, Path};

    pub struct ApiDoc;

    impl OpenApi for ApiDoc {
        fn openapi() -> utoipa::openapi::OpenApi {
            use utoipa::openapi::{InfoBuilder, OpenApiBuilder};

            OpenApiBuilder::new()
                .info(
                    InfoBuilder::new()
                        .title("Bookmarks API")
                        .description(Some("Bookmarks API"))
                        .version("1.0")
                        .build(),
                )
                .paths(bearmark_macro::utoipa_paths!(
                    "/api/bookmarks",
                    create_bookmark,
                    search_bookmarks,
                    delete_bookmark,
                    update_bookmark
                ))
                .components(Some(bearmark_macro::utoipa_components![
                    CreateBookmark,
                    ModifyBookmark,
                    Bookmark
                ]))
                .build()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::configs::{self, Config};
    use crate::db::bookmark::test::rand_bookmark;
    use crate::utils::rand::rand_str;

    use itertools::Itertools;
    use rocket::fairing::AdHoc;
    use rocket::http::Status;
    use rocket::local::{asynchronous, blocking};
    use rocket_db_pools::Database;
    use tracing::info;

    fn test_app() -> rocket::Rocket<rocket::Build> {
        rocket::custom(configs::config_provider())
            .attach(Db::init())
            .mount("/", routes())
            .attach(AdHoc::config::<Config>())
    }

    fn test_client() -> blocking::Client {
        blocking::Client::tracked(test_app()).expect("valid rocket instance")
    }

    async fn test_async_client() -> asynchronous::Client {
        asynchronous::Client::tracked(test_app())
            .await
            .expect("valid rocket instance")
    }

    #[test]
    fn create_bookmark() {
        let client = test_client();
        let payload = CreateBookmark {
            url: "https://www.rust-lang.org".to_string(),
            title: "Rust".to_string(),
            folder_id: None,
            tags: vec![rand_str(4), rand_str(4)],
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
        let payload = CreateBookmark {
            url: "https://www.rust-lang.org".to_string(),
            title: "Rust".to_string(),
            folder_id: None,
            tags: vec![rand_str(4), rand_str(4)],
        };
        let response = client
            .post(uri!(super::create_bookmark))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let added: Bookmark = response.into_json().unwrap();

        let response = client
            .delete(uri!(super::delete_bookmark(added.id)))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client
            .delete(uri!(super::delete_bookmark(added.id)))
            .dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn search_bookmarks() {
        // Create some bookmarks
        let mut conn = crate::db::connection::establish().await;
        crate::db::search::test::setup_searchable_bookmarks(&mut conn).await;

        let client = test_async_client().await;
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
            uri!(super::search_bookmarks(
                q = _,
                cwd = _,
                before = _,
                limit = _
            )),
            results.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("Weather"),
                cwd = _,
                before = _,
                limit = _
            )),
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("Weather"),
                cwd = _,
                before = _,
                limit = Some(2)
            )),
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("Weather"),
                cwd = _,
                before = Some(results[1].id),
                limit = Some(2)
            )),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("#global weather"),
                cwd = _,
                before = _,
                limit = _
            )),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("#west weather"),
                cwd = _,
                before = _,
                limit = _
            )),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("#global #west weather"),
                cwd = _,
                before = _,
                limit = _
            )),
            results.is_empty(),
            "Expected 0 bookmark, got {}",
            results.len()
        );

        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("#weather"),
                cwd = _,
                before = _,
                limit = _
            )),
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );
        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("#weather"),
                cwd = _,
                before = _,
                limit = Some(1)
            )),
            results.len() == 1,
            "Expected 1 bookmark, got {}",
            results.len()
        );
        assert_get_bookmarks!(
            uri!(super::search_bookmarks(
                q = Some("#weather"),
                cwd = _,
                before = Some(results[0].id),
                limit = Some(3)
            )),
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );
    }

    #[test]
    fn unsearchable_deleted_bookmark() {
        let payload = rand_bookmark();
        let payload = CreateBookmark {
            url: payload.url,
            title: payload.title,
            folder_id: None,
            tags: vec![rand_str(4), rand_str(4)],
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
                let response = client.get(
                    uri!(super::search_bookmarks(
                        q = Some(&title),
                        cwd = _,
                        before = _,
                        limit = _
                    ))
                ).dispatch();
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

        let response = client
            .delete(uri!(super::delete_bookmark(id = added.id)))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        assert_get_bookmark!(
            results.is_empty(),
            "Expected 0 bookmarks, got {}",
            results.len()
        );
    }

    #[test]
    fn update_exist_bookmark() {
        let client = test_client();
        let m = rand_bookmark();
        let payload = CreateBookmark {
            url: m.url,
            title: m.title,
            folder_id: None,
            tags: vec![rand_str(4), rand_str(4)],
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
            .patch(uri!(super::update_bookmark(added.id)))
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

        let response = client
            .patch(uri!(super::update_bookmark(99999999)))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    fn update_bookmark_no_change() {
        let client = test_client();
        let m = rand_bookmark();
        let payload = CreateBookmark {
            url: m.url,
            title: m.title,
            folder_id: None,
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
            .patch(uri!(super::update_bookmark(added.id)))
            .json(&payload)
            .dispatch();
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[test]
    fn update_bookmark_tags() {
        let client = test_client();
        let m = rand_bookmark();
        let payload = CreateBookmark {
            url: m.url,
            title: m.title,
            folder_id: None,
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
            .patch(uri!(super::update_bookmark(added.id)))
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
