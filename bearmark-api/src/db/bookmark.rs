use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use bearmark_types::schema::bookmarks;

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

pub async fn create_bookmark(conn: &mut Connection, new_bookmark: &NewBookmark) -> Bookmark {
    diesel::insert_into(bookmarks::table)
        .values(new_bookmark)
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
    use diesel::{ExpressionMethods, dsl::now};
    diesel::update(bookmarks::table.find(id))
        .set((&modified, bookmarks::updated_at.eq(now)))
        .returning(Bookmark::as_returning())
        .get_result(conn)
        .await
        .optional()
        .expect("Error updating bookmark")
}

pub async fn delete_bookmarks(conn: &mut Connection, ids: Vec<i32>) -> usize {
    use diesel::{ExpressionMethods, dsl::now};

    use bearmark_types::schema::bookmarks::{dsl::*, table};

    diesel::update(table)
        .filter(id.eq_any(ids).and(deleted_at.is_null()))
        .set((deleted_at.eq(now), updated_at.eq(now)))
        .execute(conn)
        .await
        .expect("Error deleting bookmarks")
}

#[cfg(test)]
pub(crate) mod test {
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

    pub async fn create_rand_bookmark(conn: &mut Connection) -> Bookmark {
        create_bookmark(conn, &rand_bookmark()).await
    }

    #[tokio::test]
    async fn create_new_bookmark() {
        let new = rand_bookmark();
        let mut conn = connection::establish().await;

        let m = create_bookmark(&mut conn, &new).await;

        info!("{:?}", m);
        assert!(m.id > 0);
    }

    #[tokio::test]
    async fn title_search_bookmark() {
        let mut conn = connection::establish().await;
        let new = rand_bookmark();
        let title = new.title.clone();
        create_bookmark(&mut conn, &new).await;

        let results = bookmarks::table
            .filter(bookmarks::dsl::title.like(title))
            .order_by(bookmarks::dsl::created_at.desc())
            .load::<Bookmark>(&mut conn)
            .await
            .expect("Error loading bookmarks");

        assert!(!results.is_empty());
        info!("{:?}", results[0]);
    }

    #[tokio::test]
    pub async fn delete_a_bookmark() {
        let mut conn = connection::establish().await;
        let m = create_rand_bookmark(&mut conn).await;
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let count = delete_bookmarks(&mut conn, vec![m.id]).await;
        assert!(count == 1);

        let m = Bookmark::get(&mut conn, m.id).await.unwrap();
        assert!(m.deleted_at.is_some());
    }

    #[tokio::test]
    async fn update_exists_bookmark() {
        let mut conn = connection::establish().await;
        let new = rand_bookmark();
        let bm = create_bookmark(&mut conn, &new).await;
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
