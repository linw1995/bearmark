use crate::db::tag::{self, Tag};

use super::fairings::db::Db;

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
        .await
        .into_iter()
        .collect(),
    )
}

pub fn routes() -> Vec<rocket::Route> {
    routes![search_tags,]
}
