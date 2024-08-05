use super::errors::Error;
use super::fairings::db::Db;
use crate::db::{
    bookmark::Bookmark,
    folder::{self, Folder},
};

use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_db_pools::Connection;
use tracing::info;

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NewFolder {
    pub path: String,
}

#[post("/", format = "application/json", data = "<payload>")]
pub async fn create_folder(
    mut db: Connection<Db>,
    payload: Json<NewFolder>,
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
        return Err(Error::BadRequest(
            "Parent folder does not exist".to_string(),
        ));
    }

    Ok(Json(folder::create_folder(&mut db, &path).await?))
}

#[get("/?<cwd>")]
pub async fn list_folders(mut db: Connection<Db>, cwd: Option<&str>) -> Json<Vec<Folder>> {
    Json(folder::list_folders(&mut db, cwd.unwrap_or_default()).await)
}

#[put("/move_in/<bookmark_id>/<id>")]
pub async fn move_bookmark(mut db: Connection<Db>, bookmark_id: i32, id: i32) -> Result<(), Error> {
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

#[put("/move_out/<bookmark_id>")]
pub async fn move_out_bookmark(mut db: Connection<Db>, bookmark_id: i32) -> Result<(), Error> {
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

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::db::bookmark::test::create_rand_bookmark;
    use crate::db::connection;
    use crate::db::folder::test::create_rand_folder;
    use crate::utils::rand::rand_str;

    use rocket::http::Status;
    use rocket::local::asynchronous;
    use rocket::local::blocking;
    use rocket_db_pools::Database;
    use tracing::info;

    fn test_client() -> blocking::Client {
        let app = rocket::build().attach(Db::init()).mount("/", routes());
        blocking::Client::tracked(app).expect("valid rocket instance")
    }

    async fn test_async_client() -> asynchronous::Client {
        let app = rocket::build().attach(Db::init()).mount("/", routes());
        asynchronous::Client::tracked(app)
            .await
            .expect("valid rocket instance")
    }

    #[test]
    fn create_new_folder() {
        let client = test_client();
        let path = format!("/{}", rand_str(10));
        let res = client
            .post(uri!(create_folder))
            .json(&NewFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let folder: Folder = res.into_json().unwrap();
        assert_eq!(folder.path, path);

        // duplicate folder
        let res = client
            .post(uri!(create_folder))
            .json(&NewFolder { path: path.clone() })
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
            .json(&NewFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);

        // create parent folder, then child folder
        let res = client
            .post(uri!(create_folder))
            .json(&NewFolder {
                path: parent_path.clone(),
            })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, parent_path);
        let res = client
            .post(uri!(create_folder))
            .json(&NewFolder { path: path.clone() })
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
            .json(&NewFolder {
                path: parent_path.clone(),
            })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, parent_path);
        let res = client
            .post(uri!(create_folder))
            .json(&NewFolder { path: path.clone() })
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Folder>().unwrap().path, path);

        let res = client.get("/").dispatch();
        assert_eq!(res.status(), Status::Ok);
        let folders: Vec<Folder> = res.into_json().unwrap();
        assert!(folders.len() > 0);
        assert!(folders.iter().any(|f| f.path == parent_path));
        assert!(folders.iter().all(|f| f.path != path));

        let res = client.get(format!("/?cwd={}", parent_path)).dispatch();
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
        let res = client.put(format!("/move_out/{}", bm.id)).dispatch().await;
        assert_eq!(res.status(), Status::Ok);

        // move out again
        let res = client.put(format!("/move_out/{}", bm.id)).dispatch().await;
        assert_eq!(res.status(), Status::BadRequest);
    }
}
