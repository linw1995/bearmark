use super::errors::Error;
use super::fairings::db::Db;
use crate::api::guards;
use crate::db::{
    bookmark::Bookmark,
    folder::{self, Folder},
};

use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_db_pools::Connection;
use tracing::info;
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateFolder {
    pub path: String,
}

/// Create a new folder
#[utoipa::path(
    post,
    path = "/",
    request_body = CreateFolder,
    responses(
        (status = 200, description = "Folder created success", body = Folder),
        (status = 400, description = "Folder already exists"),
        (status = 404, description = "Parent folder does not exist")
    ),
    security(
        ("api_key" = [])
    )
)]
#[post("/", format = "application/json", data = "<payload>")]
pub async fn create_folder(
    mut db: Connection<Db>,
    _required: guards::Auth,
    payload: Json<CreateFolder>,
) -> Result<Json<Folder>, Error> {
    let path = payload.into_inner().path;

    let mut self_and_ancestors = Folder::get_with_ancestors(&mut db, &path).await.into_iter();

    // check if folder already exists
    if let Some(Some(folder)) = self_and_ancestors.next() {
        return Err(Error::BadRequest(format!(
            "Folder already exists: {}",
            folder.path
        )));
    }

    // check if any parent folder does not exist
    if self_and_ancestors.any(|f| f.is_none()) {
        return Err(Error::NotFound("Parent folder does not exist".to_string()));
    }

    Ok(Json(folder::create_folder(&mut db, &path).await?))
}

/// List folders
#[utoipa::path(
    get,
    path = "/",
    params(
        ("cwd" = inline(Option<&str>), Query, description = "The path of folder to search in"),
    ),
    responses(
        (status = 200, description = "Folders searched success", body = Vec<Folder>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[get("/?<cwd>")]
pub async fn list_folders(
    mut db: Connection<Db>,
    _required: guards::Auth,
    cwd: Option<&str>,
) -> Json<Vec<Folder>> {
    Json(folder::list_folders(&mut db, cwd.unwrap_or_default()).await)
}

/// Move a bookmark into a folder
#[utoipa::path(
    get,
    path = "/move_in/{bookmark_id}/{id}",
    params(
        ("bookmark_id" = inline(i32), Query, description = "The id of target bookmark"),
        ("id" = inline(i32), Query, description = "The id of target folder"),
    ),
    responses(
        (status = 200, description = "Bookmark moved into folder success")
    ),
    security(
        ("api_key" = [])
    )
)]
#[put("/move_in/<bookmark_id>/<id>")]
pub async fn move_bookmark(
    mut db: Connection<Db>,
    _required: guards::Auth,
    bookmark_id: i32,
    id: i32,
) -> Result<(), Error> {
    let b = Bookmark::get(&mut db, bookmark_id)
        .await
        .ok_or(Error::NotFound("Bookmark not found".to_string()))?;
    if let Some(folder_id) = b.folder_id {
        if folder_id == id {
            return Err(Error::BadRequest(
                "Bookmark is already in this folder".to_string(),
            ));
        }
        info!(?bookmark_id, from = folder_id, to = id, "Moving bookmark");
    } else {
        info!(?bookmark_id, to = id, "Moving bookmark");
    }
    let _ = Folder::get(&mut db, id)
        .await
        .ok_or(Error::NotFound("Folder not found".to_string()))?;
    Ok(folder::move_bookmarks(&mut db, id, &vec![bookmark_id]).await?)
}

/// Move a bookmark out of a folder
#[utoipa::path(
    get,
    path = "/move_out/{bookmark_id}",
    params(
        ("bookmark_id" = inline(i32), Query, description = "The id of target bookmark"),
    ),
    responses(
        (status = 200, description = "Bookmark moved out of folder success")
    ),
    security(
        ("api_key" = [])
    )
)]
#[put("/move_out/<bookmark_id>")]
pub async fn move_out_bookmark(
    mut db: Connection<Db>,
    _required: guards::Auth,
    bookmark_id: i32,
) -> Result<(), Error> {
    let folder_id = Bookmark::get(&mut db, bookmark_id)
        .await
        .ok_or(Error::NotFound("Bookmark not found".to_string()))?
        .folder_id
        .ok_or(Error::BadRequest("Bookmark is not in a folder".to_string()))?;
    info!(?bookmark_id, ?folder_id, "Moving out bookmark");
    folder::move_out_bookmarks(&mut db, &vec![bookmark_id]).await;
    Ok(())
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        list_folders,
        create_folder,
        move_bookmark,
        move_out_bookmark
    ]
}

#[cfg(not(tarpaulin_include))]
pub(crate) mod misc {
    use super::*;

    use utoipa::{OpenApi, Path};

    pub struct ApiDoc;

    impl OpenApi for ApiDoc {
        fn openapi() -> utoipa::openapi::OpenApi {
            use utoipa::openapi::{
                security::{ApiKey, ApiKeyValue, SecurityScheme},
                InfoBuilder, OpenApiBuilder,
            };

            let mut api = OpenApiBuilder::new()
                .info(
                    InfoBuilder::new()
                        .title("Folders API")
                        .description(Some("Folders API"))
                        .version("1.0")
                        .build(),
                )
                .paths(bearmark_macro::utoipa_paths!(
                    "/api/folders",
                    create_folder,
                    list_folders,
                    move_bookmark,
                    move_out_bookmark
                ))
                .components(Some(bearmark_macro::utoipa_components![
                    CreateFolder,
                    Folder,
                    Error
                ]))
                .build();

            api.components.as_mut().unwrap().add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("Authorization"))),
            );

            api
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::api::configs;
    use crate::api::configs::Config;
    use crate::db::bookmark::test::create_rand_bookmark;
    use crate::db::connection;
    use crate::db::folder::test::create_rand_folder;
    use crate::utils::rand::rand_str;

    use rocket::fairing::AdHoc;
    use rocket::http::Status;
    use rocket::local::asynchronous;
    use rocket::local::blocking;
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
    fn create_new_folder() {
        let client = test_client();
        let path = format!("/{}", rand_str(10));
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let folder: Folder = res.into_json().unwrap();
        assert_eq!(folder.path, path);

        // duplicate folder
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        info!("duplicate folder response: {:?}", res.into_string());
    }

    #[test]
    fn create_nested_folders() {
        let client = test_client();
        let parent_path = format!("/{}", rand_str(10));
        let path = format!("{}/{}", parent_path, rand_str(10));

        // need to create parent folder first
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);

        // create parent folder, then child folder
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder {
                path: parent_path.clone(),
            })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, parent_path);
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, path);
    }

    #[test]
    fn list_folders_test() {
        let client = test_client();

        // create nested folders
        let parent_path = format!("/{}", rand_str(10));
        let path = format!("{}/{}", parent_path, rand_str(10));
        // create parent folder, then child folder
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder {
                path: parent_path.clone(),
            })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, parent_path);
        let res = client
            .post(uri!(create_folder))
            .json(&CreateFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, path);

        let res = client.get(uri!(super::list_folders(cwd = _))).dispatch();
        assert_eq!(res.status(), Status::Ok);
        let folders: Vec<Folder> = res.into_json().unwrap();
        assert!(!folders.is_empty());
        assert!(folders.iter().any(|f| f.path == parent_path));
        assert!(folders.iter().all(|f| f.path != path));

        let res = client
            .get(uri!(super::list_folders(cwd = Some(parent_path))))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let folders: Vec<Folder> = res.into_json().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].path, path);
    }

    #[rocket::async_test]
    async fn bookmarks_movements() {
        let mut conn = connection::establish().await;
        let bm = create_rand_bookmark(&mut conn).await;
        let f01 = create_rand_folder(&mut conn).await;
        let f02 = create_rand_folder(&mut conn).await;

        let client = test_async_client().await;
        let res = client
            .put(format!("/move_in/{}/{}", bm.id, f01.id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // move in again
        let res = client
            .put(format!("/move_in/{}/{}", bm.id, f01.id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::BadRequest);

        // move in to another folder
        let res = client
            .put(format!("/move_in/{}/{}", bm.id, f02.id))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // move out
        let res = client
            .put(uri!(super::move_out_bookmark(bm.id)))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        // move out again
        let res = client
            .put(uri!(super::move_out_bookmark(bm.id)))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::BadRequest);
    }
}
