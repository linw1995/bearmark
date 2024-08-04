use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use super::schema::{bookmarks, folders};
use crate::utils::DatabaseError;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Deserialize, Serialize)]
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

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = folders)]
pub struct NewFolder<'a> {
    pub path: &'a str,
}

impl Folder {
    pub async fn get(conn: &mut Connection, id: i32) -> Option<Self> {
        folders::table
            .find(id)
            .first(conn)
            .await
            .optional()
            .expect("Error loading folder")
    }

    pub async fn get_by_path(conn: &mut Connection, path: &str) -> Option<Self> {
        folders::table
            .filter(folders::dsl::path.eq(path))
            .first(conn)
            .await
            .optional()
            .expect("Error loading folder")
    }

    pub async fn get_with_ancestors(conn: &mut Connection, path: &str) -> Vec<Option<Self>> {
        let mut ancestors = Vec::new();
        let mut path = path.trim_matches('/');
        while !path.is_empty() {
            ancestors.push(Self::get_by_path(conn, &format!("/{}", path)).await);
            let mut split = path.rsplitn(2, '/');
            // result is in reverse order, so nth(1) is the parent, nth(0) is the current
            // and if nth(1) is None, then it's the root folder
            if let Some(parent) = split.nth(1) {
                path = parent;
            } else {
                break;
            }
        }
        ancestors
    }
}

pub async fn create_folder(conn: &mut Connection, path: &str) -> Result<Folder, DatabaseError> {
    diesel::insert_into(folders::table)
        .values(&NewFolder {
            // path should be normalized, start with `/` and end without `/`
            path: format!("/{}", path.trim_matches('/')).as_str(),
        })
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

pub async fn delete_folder(conn: &mut Connection, id: i32) {
    diesel::delete(folders::table.filter(folders::dsl::id.eq(id)))
        .execute(conn)
        .await
        .expect("Error deleting folder");
}

pub async fn move_bookmarks(
    conn: &mut Connection,
    folder_id: i32,
    bookmark_ids: &Vec<i32>,
) -> Result<(), DatabaseError> {
    diesel::update(bookmarks::table)
        .filter(bookmarks::dsl::id.eq_any(bookmark_ids))
        .set(bookmarks::dsl::folder_id.eq(folder_id))
        .execute(conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _,
            ) => DatabaseError::ViolationError(),
            _ => panic!("Unexpected error: {:?}", e),
        })?;

    Ok(())
}

pub async fn move_out_bookmarks(conn: &mut Connection, bookmark_ids: &Vec<i32>) {
    diesel::update(bookmarks::table)
        .filter(bookmarks::dsl::id.eq_any(bookmark_ids))
        .set(bookmarks::dsl::folder_id.eq::<Option<i32>>(None))
        .execute(conn)
        .await
        .expect("Error moving out bookmarks");
}

pub async fn list_folders(conn: &mut Connection, cwd: &str) -> Vec<Folder> {
    use super::extending::RegexMatchExtensions;

    folders::table
        .select(Folder::as_select())
        .filter(folders::dsl::path.regex_match(format!("^{}/[^/]*$", cwd.trim_end_matches('/'))))
        .load::<Folder>(conn)
        .await
        .expect("Error loading folders")
}

#[cfg(test)]
pub mod tests {
    use super::super::connection;
    use super::*;
    use crate::db::bookmark::test::rand_bookmark;
    use crate::db::bookmark::{create_bookmark, Bookmark};
    use crate::utils::rand::rand_str;

    use futures::future::join_all;
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

    #[tokio::test]
    async fn bookmarks_movements() {
        let mut conn = connection::establish().await;

        let path = format!("/{}", rand_str(10));
        let folder = create_folder(&mut conn, &path).await.unwrap();
        info!(?folder, "folder created");

        let bookmark_ids = join_all((0..10).map(|_| async {
            let mut conn = connection::establish().await;
            let bm = rand_bookmark();
            let bm = create_bookmark(&mut conn, bm).await;
            bm.id
        }))
        .await;

        info!(?bookmark_ids, "bookmarks move into folder");
        let rv = move_bookmarks(&mut conn, folder.id, &bookmark_ids).await;
        assert!(rv.is_ok());
        for id in &bookmark_ids {
            let bm = Bookmark::get(&mut conn, *id).await.unwrap();
            assert_eq!(bm.folder_id, Some(folder.id));
        }

        info!(?bookmark_ids, "bookmarks move out of folder");
        move_out_bookmarks(&mut conn, &bookmark_ids).await;
        for id in &bookmark_ids {
            let bm = Bookmark::get(&mut conn, *id).await.unwrap();
            assert_eq!(bm.folder_id, None);
        }

        info!("folder deleted");
        delete_folder(&mut conn, folder.id).await;

        info!("bookmarks move into deleted folder");
        let rv = move_bookmarks(&mut conn, folder.id, &bookmark_ids).await;
        assert!(rv.is_err());
    }
}
