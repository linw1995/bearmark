use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use super::bookmark::Bookmark;
use super::schema::{bookmarks_folders, folders};
use crate::utils::DatabaseError;

#[derive(Queryable, Selectable, Identifiable, Debug, Deserialize, Serialize)]
#[diesel(table_name = folders)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Folder {
    pub id: i32,
    pub path: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
}

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug)]
#[diesel(belongs_to(Bookmark))]
#[diesel(belongs_to(Folder))]
#[diesel(table_name = bookmarks_folders)]
#[diesel(primary_key(bookmark_id, folder_id))]
pub struct BookmarkFolder {
    pub bookmark_id: i32,
    pub folder_id: i32,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = folders)]
pub struct NewFolder<'a> {
    pub path: &'a str,
}

pub async fn create_folder(conn: &mut Connection, path: &str) -> Result<Folder, DatabaseError> {
    diesel::insert_into(folders::table)
        .values(&NewFolder { path })
        .returning(Folder::as_returning())
        .get_result(conn)
        .await
        .map_err(|e: diesel::result::Error| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => DatabaseError::DuplicationError {
                table: "folders".to_string(),
            },
            _ => panic!("Unexpected error: {:?}", e),
        })
}

#[cfg(test)]
pub mod tests {
    use super::super::connection;
    use super::*;
    use crate::utils::rand::rand_str;

    use tracing::info;

    #[tokio::test]
    async fn create_new_folder() {
        let mut conn = connection::establish().await;

        let path = format!("/{}", rand_str(10));
        let rv = create_folder(&mut conn, &path).await;
        info!(?rv, "create_folder returns");
        assert!(rv.is_ok());

        let rv = create_folder(&mut conn, &path).await;
        info!(?rv, "create_folder returns");
        assert!(rv.is_err());
        assert!(matches!(
            rv.unwrap_err(),
            DatabaseError::DuplicationError { .. }
        ));
    }
}
