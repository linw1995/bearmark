use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use super::schema::bookmarks;

#[derive(
    Queryable,
    Selectable,
    Identifiable,
    AsChangeset,
    Deserialize,
    Serialize,
    Hash,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
#[diesel(table_name = bookmarks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Bookmark {
    pub id: i32,
    pub title: String,
    pub url: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
    pub folder_id: Option<i32>,
}

#[derive(Insertable, AsChangeset, Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = bookmarks)]
pub struct NewBookmark {
    pub title: String,
    pub url: String,
}

#[derive(AsChangeset, Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = bookmarks)]
pub struct ModifyBookmark {
    pub title: Option<String>,
    pub url: Option<String>,
}

impl Bookmark {
    pub async fn get(conn: &mut Connection, id: i32) -> Option<Bookmark> {
        bookmarks::table
            .find(id)
            .first(conn)
            .await
            .optional()
            .expect("Error loading bookmark")
    }
}

pub async fn search_bookmarks(
    conn: &mut Connection,
    keywords: &Vec<&str>,
    before: i32,
    limit: i64,
) -> Vec<Bookmark> {
    // Cursor-based pagination
    // before: id of the last bookmark in the previous page. 0 for the first page.

    let mut query = bookmarks::table
        .filter(bookmarks::dsl::deleted_at.is_null())
        .into_boxed();

    for keyword in keywords {
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
        .expect("Error search bookmarks")
}

pub async fn create_bookmark(conn: &mut Connection, new_bookmark: NewBookmark) -> Bookmark {
    diesel::insert_into(bookmarks::table)
        .values(&new_bookmark)
        .returning(Bookmark::as_returning())
        .get_result(conn)
        .await
        .expect("Error saving new bookmark")
}

pub async fn update_bookmark(
    conn: &mut Connection,
    id: i32,
    modified: ModifyBookmark,
) -> Option<Bookmark> {
    use diesel::{dsl::now, ExpressionMethods};
    diesel::update(bookmarks::table.find(id))
        .set((&modified, bookmarks::updated_at.eq(now)))
        .returning(Bookmark::as_returning())
        .get_result(conn)
        .await
        .optional()
        .expect("Error updating bookmark")
}

pub async fn delete_bookmarks(conn: &mut Connection, ids: Vec<i32>) -> usize {
    use diesel::{dsl::now, ExpressionMethods};

    use super::schema::bookmarks::{dsl::*, table};

    diesel::update(table)
        .filter(id.eq_any(ids).and(deleted_at.is_null()))
        .set((deleted_at.eq(now), updated_at.eq(now)))
        .execute(conn)
        .await
        .expect("Error deleting bookmarks")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::connection;
    use super::*;
    use crate::utils;
    use tracing::info;

    pub fn rand_bookmark() -> NewBookmark {
        NewBookmark {
            title: utils::rand::rand_str(10),
            url: format!("https://{}.com", utils::rand::rand_str(10)).to_string(),
        }
    }

    #[tokio::test]
    async fn create_new_bookmark() {
        let new_bookmark = rand_bookmark();
        let mut conn = connection::establish().await;

        let m = create_bookmark(&mut conn, new_bookmark).await;

        info!("{:?}", m);
        assert!(m.id > 0);
    }

    #[tokio::test]
    async fn title_search_bookmark() {
        let mut conn = connection::establish().await;
        let new_bookmark = rand_bookmark();
        let title = new_bookmark.title.clone();
        create_bookmark(&mut conn, new_bookmark).await;

        let results = bookmarks::table
            .filter(bookmarks::dsl::title.like(title))
            .order_by(bookmarks::dsl::created_at.desc())
            .load::<Bookmark>(&mut conn)
            .await
            .expect("Error loading bookmarks");

        assert!(results.len() > 0);
        info!("{:?}", results[0]);
    }

    #[tokio::test]
    pub async fn delete_a_bookmark() {
        let mut conn = connection::establish().await;
        let new_bookmark = rand_bookmark();
        let m = create_bookmark(&mut conn, new_bookmark).await;
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let count = delete_bookmarks(&mut conn, vec![m.id]).await;
        assert!(count == 1);

        let m = Bookmark::get(&mut conn, m.id).await.unwrap();
        assert!(m.deleted_at.is_some());
    }

    pub async fn setup_searchable_bookmarks(conn: &mut Connection) {
        let values = vec![
            NewBookmark {
                title: "Weather".to_string(),
                url: "https://example.com".to_string(),
            },
            NewBookmark {
                title: "News".to_string(),
                url: "https://example.com".to_string(),
            },
            NewBookmark {
                title: "Social".to_string(),
                url: "https://example.com".to_string(),
            },
            NewBookmark {
                title: "Weather Global".to_string(),
                url: "https://example.com".to_string(),
            },
            NewBookmark {
                title: "Weather West".to_string(),
                url: "https://example.com".to_string(),
            },
        ];

        // delete bookmarks with same title
        let titles = values
            .iter()
            .map(|v| v.title.clone())
            .collect::<Vec<String>>();
        diesel::delete(bookmarks::table)
            .filter(bookmarks::title.eq_any(titles))
            .execute(conn)
            .await
            .expect("Error deleting bookmarks");

        diesel::insert_into(bookmarks::table)
            .values(&values)
            .execute(conn)
            .await
            .expect("Error saving new bookmarks");
    }

    #[tokio::test]
    #[file_serial] // For allowing remove data of table in test
    pub async fn search_bookmarks_with_pagination() {
        let mut conn = connection::establish().await;
        setup_searchable_bookmarks(&mut conn).await;

        let results = search_bookmarks(&mut conn, &vec![], 0, 10).await;
        assert!(
            results.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks(&mut conn, &vec!["Weather"], 0, 10).await;
        assert!(
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks(&mut conn, &vec!["Weather"], 0, 2).await;
        assert!(
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks(&mut conn, &vec!["Weather"], results[1].id, 2).await;
        assert!(
            results.len() == 1,
            "Expected 1 bookmarks, got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn unsearchable_deleted_bookmark() {
        let mut conn = connection::establish().await;
        let new_bookmark = rand_bookmark();
        let title = new_bookmark.title.clone();
        let m = create_bookmark(&mut conn, new_bookmark).await;
        info!(?m, "created");
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let result = search_bookmarks(&mut conn, &vec![&title], 0, 1).await;
        info!(?result, "searched");
        assert!(result.len() == 1);

        let count = delete_bookmarks(&mut conn, vec![m.id]).await;
        assert!(count == 1);

        let result = search_bookmarks(&mut conn, &vec![&title], 0, 1).await;
        info!(?result, "searched");
        assert!(result.len() == 0);
    }

    #[tokio::test]
    async fn update_exists_bookmark() {
        let mut conn = connection::establish().await;
        let new = rand_bookmark();
        let bm = create_bookmark(&mut conn, new.clone()).await;
        assert!(bm.id > 0);
        assert_eq!(bm.title, new.title);
        assert_eq!(bm.url, new.url);

        let modified = rand_bookmark();

        assert_ne!(new.title, modified.title);
        assert_ne!(new.url, modified.url);

        let rv = update_bookmark(
            &mut conn,
            bm.id,
            ModifyBookmark {
                title: Some(modified.title.clone()),
                url: Some(modified.url.clone()),
            },
        )
        .await;
        assert!(rv.is_some());
        let modified_bm = rv.unwrap();
        assert_eq!(modified_bm.id, bm.id);
        assert_eq!(modified_bm.title, modified.title);
        assert_eq!(modified_bm.url, modified.url);
    }
}
