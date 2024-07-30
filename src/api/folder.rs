use super::errors::Error;
use super::fairings::db::Db;
use crate::db::folder::{self, Folder};

use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_db_pools::Connection;

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
    Ok(Json(
        folder::create_folder(&mut db, payload.into_inner().path.as_str()).await?,
    ))
}

#[get("/?<cwd>")]
pub async fn list_folders(mut db: Connection<Db>, cwd: Option<&str>) -> Json<Vec<Folder>> {
    Json(folder::list_folders(&mut db, cwd.unwrap_or_default()).await)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![list_folders, create_folder]
}
