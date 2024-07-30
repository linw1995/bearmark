use super::errors::Error;
use super::fairings::db::Db;
use crate::db::tag::{self, Tag};

use rocket::serde::json::Json;
use rocket_db_pools::Connection;

#[get("/?<q>&<before>&<limit>")]
pub async fn search_tags(
    mut db: Connection<Db>,
    q: Option<&str>,
    before: Option<i32>,
    limit: Option<i64>,
) -> Json<Vec<Tag>> {
    let keywords = q.map(|q| vec![q.trim()]).unwrap_or_default();
    Json(
        tag::search_tags(
            &mut db,
            &keywords,
            before.unwrap_or_default(),
            limit.unwrap_or(10),
        )
        .await,
    )
}

#[delete("/<id>")]
pub async fn delete_tag(mut db: Connection<Db>, id: i32) -> Result<&'static str, Error> {
    let effected = tag::delete_tags(&mut db, vec![id]).await == 1;
    if effected {
        Ok("Deleted")
    } else {
        Err(Error::NotFound("Tag not found".to_string()))
    }
}

#[patch("/<id>", format = "application/json", data = "<payload>")]
pub async fn update_tag(
    mut db: Connection<Db>,
    id: i32,
    payload: Json<tag::ModifyTag>,
) -> Result<Json<Tag>, Error> {
    let payload = payload.into_inner();
    if payload.name.is_none() {
        return Err(Error::BadRequest("No changes".to_string()));
    }
    tag::update_tag(&mut db, id, payload)
        .await
        .ok_or_else(|| Error::NotFound("Tag not found".to_string()))
        .map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![search_tags, delete_tag, update_tag]
}
