use diesel::dsl::InnerJoinQuerySource;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use super::bookmark::{self, Bookmark};
use super::schema::{bookmarks, folders};
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

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = folders)]
pub struct NewFolder<'a> {
    pub path: &'a str,
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

pub async fn search_bookmarks(
    conn: &mut Connection,
    keywords: &Vec<&str>,
    paths: &[&str],
    before: i32,
    limit: i64,
) -> Vec<Bookmark> {
    if paths.is_empty() {
        bookmark::search_bookmarks(conn, keywords, before, limit).await
    } else {
        use super::schema::bookmarks;
        use diesel::BoolExpressionMethods;

        let mut query = bookmarks::table
            .inner_join(folders::table)
            .select(Bookmark::as_select())
            .filter(bookmarks::dsl::deleted_at.is_null())
            .into_boxed();

        macro_rules! filter_folder_and_descendants {
            ($path: expr) => {{
                let v = $path;
                folders::dsl::path
                    .eq(v)
                    .or(folders::dsl::path.like(format!("{}/%", v)))
            }};
        }

        let mut iter = paths.iter();
        let init = Box::new(filter_folder_and_descendants!(iter.next().unwrap()));
        let condition: Box<
            dyn BoxableExpression<
                InnerJoinQuerySource<bookmarks::table, folders::table>,
                _,
                SqlType = Bool,
            >,
        > = iter.fold(init, |acc, path| {
            Box::new(acc.or(filter_folder_and_descendants!(path)))
        });
        query = query.filter(condition);

        for keyword in keywords {
            if keyword.is_empty() {
                continue;
            }
            query = query.filter(
                bookmarks::dsl::title
                    .ilike(format!("%{}%", keyword))
                    .or(bookmarks::dsl::url.ilike(format!("%{}%", keyword))),
            )
        }

        if before > 0 {
            query = query.filter(bookmarks::dsl::id.lt(before))
        }

        query
            .order_by(bookmarks::dsl::id.desc())
            .limit(limit)
            .load::<Bookmark>(conn)
            .await
            .expect("Error loading bookmarks")
    }
}

#[cfg(test)]
pub mod tests {
    use super::super::connection;
    use super::*;
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
            let bm = bookmark::tests::rand_bookmark();
            let bm = bookmark::create_bookmark(&mut conn, bm).await;
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

    #[tokio::test]
    async fn search_bookmarks_in_folders() {
        let mut conn = connection::establish().await;

        let folder1_path = format!("/{}", rand_str(10));
        let folder2_path = format!("/{}", rand_str(10));
        let folder3_path = format!("{}/{}", folder1_path, rand_str(10));
        let folder1 = create_folder(&mut conn, &folder1_path).await.unwrap();
        let folder2 = create_folder(&mut conn, &folder2_path).await.unwrap();
        let folder3 = create_folder(&mut conn, &folder3_path).await.unwrap();

        let bookmark_ids = join_all((0..10).map(|_| async {
            let mut conn = connection::establish().await;
            let bm = bookmark::tests::rand_bookmark();
            let bm = bookmark::create_bookmark(&mut conn, bm).await;
            bm.id
        }))
        .await;

        move_bookmarks(&mut conn, folder1.id, &bookmark_ids)
            .await
            .unwrap();

        let bm = bookmark::tests::rand_bookmark();
        let bm = bookmark::create_bookmark(&mut conn, bm).await;
        let bookmark_ids = vec![bm.id];

        move_bookmarks(&mut conn, folder2.id, &bookmark_ids)
            .await
            .unwrap();

        info!("search bookmarks in folder1");
        let rv = search_bookmarks(&mut conn, &vec![""], &[&folder1_path], 0, 10).await;
        assert_eq!(rv.len(), 10);

        info!("search bookmarks in folder2");
        let rv = search_bookmarks(&mut conn, &vec![""], &[&folder2_path], 0, 10).await;
        assert_eq!(rv.len(), 1);

        info!("search bookmarks in folder1 and folder2");
        let rv = search_bookmarks(
            &mut conn,
            &vec![""],
            &[&folder1_path, &folder2_path],
            0,
            100,
        )
        .await;
        assert_eq!(rv.len(), 11);

        let rv =
            search_bookmarks(&mut conn, &vec![""], &[&folder1_path, &folder2_path], 0, 5).await;
        assert_eq!(rv.len(), 5);

        let rv =
            search_bookmarks(&mut conn, &vec![""], &[&folder1_path, &folder2_path], 0, 1).await;
        assert_eq!(rv.len(), 1);

        info!("search bookmarks in folder1 and its descendants, folder3");
        let bm = bookmark::tests::rand_bookmark();
        let bm = bookmark::create_bookmark(&mut conn, bm).await;
        let bookmark_ids = vec![bm.id];
        move_bookmarks(&mut conn, folder3.id, &bookmark_ids)
            .await
            .unwrap();
        let rv = search_bookmarks(&mut conn, &vec![""], &[&folder1_path], 0, 100).await;
        assert_eq!(rv.len(), 11);
    }
}
