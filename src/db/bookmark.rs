use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use super::schema::bookmarks;

#[derive(Queryable, Selectable, AsChangeset, Debug, Deserialize, Serialize)]
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
    title: &str,
    before: i32,
    limit: i64,
) -> Vec<Bookmark> {
    // Cursor-based pagination
    // before: id of the last bookmark in the previous page. 0 for the first page.

    let mut query = bookmarks::table
        .filter(bookmarks::dsl::deleted_at.is_null())
        .into_boxed();

    if !title.is_empty() {
        query = query.filter(bookmarks::dsl::title.like(format!("%{}%", title)))
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

pub async fn delete_bookmarks(conn: &mut Connection, ids: Vec<i32>) -> usize {
    use diesel::{dsl::now, ExpressionMethods};

    use super::schema::bookmarks::{dsl::*, table};

    diesel::update(table)
        .filter(id.eq_any(ids).and(deleted_at.is_null()))
        .set(deleted_at.eq(now))
        .execute(conn)
        .await
        .expect("Error deleting bookmarks")
}

#[derive(Insertable, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = bookmarks)]
pub struct NewBookmark {
    pub title: String,
    pub url: String,
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
            url: "https://example.com".to_string(),
        }
    }

    async fn clean_bookmarks(conn: &mut Connection) {
        diesel::delete(bookmarks::table)
            .execute(conn)
            .await
            .expect("Error deleting bookmarks");
    }

    #[tokio::test]
    async fn create_new_bookmark() {
        let new_bookmark = rand_bookmark();
        let mut conn = connection::establish_async().await;

        let m = create_bookmark(&mut conn, new_bookmark).await;

        info!("{:?}", m);
        assert!(m.id > 0);
    }

    #[tokio::test]
    async fn title_search_bookmark() {
        let mut conn = connection::establish_async().await;
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
        let mut conn = connection::establish_async().await;
        let new_bookmark = rand_bookmark();
        let m = create_bookmark(&mut conn, new_bookmark).await;
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let count = delete_bookmarks(&mut conn, vec![m.id]).await;
        assert!(count == 1);

        let m = Bookmark::get(&mut conn, m.id).await.unwrap();
        assert!(m.deleted_at.is_some());
    }

    #[tokio::test]
    #[serial] // For allowing remove all data of table in test
    pub async fn search_bookmarks_with_pagination() {
        let mut conn = connection::establish_async().await;
        clean_bookmarks(&mut conn).await;

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

        diesel::insert_into(bookmarks::table)
            .values(&values)
            .execute(&mut connection::establish_async().await)
            .await
            .expect("Error saving new bookmarks");

        let results = search_bookmarks(&mut conn, "", 0, 10).await;
        assert!(
            results.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks(&mut conn, "Weather", 0, 10).await;
        assert!(
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks(&mut conn, "Weather", 0, 2).await;
        assert!(
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks(&mut conn, "Weather", results[1].id, 2).await;
        assert!(
            results.len() == 1,
            "Expected 1 bookmarks, got {}",
            results.len()
        );
    }

    #[tokio::test]
    #[serial]
    async fn unsearchable_deleted_bookmark() {
        let mut conn = connection::establish_async().await;
        let new_bookmark = rand_bookmark();
        let title = new_bookmark.title.clone();
        let m = create_bookmark(&mut conn, new_bookmark).await;
        info!(?m, "created");
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let result = search_bookmarks(&mut conn, &title, 0, 1).await;
        info!(?result, "searched");
        assert!(result.len() == 1);

        let count = delete_bookmarks(&mut conn, vec![m.id]).await;
        assert!(count == 1);

        let result = search_bookmarks(&mut conn, &title, 0, 1).await;
        info!(?result, "searched");
        assert!(result.len() == 0);
    }
}
