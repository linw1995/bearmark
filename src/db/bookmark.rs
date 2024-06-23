use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};

use super::connection;
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
    pub fn get(id: i32) -> Option<Bookmark> {
        let mut conn = connection::establish();

        bookmarks::table
            .find(id)
            .first(&mut conn)
            .optional()
            .expect("Error loading bookmark")
    }
}

pub fn search_bookmarks(title: &str, before: i32, limit: i64) -> Vec<Bookmark> {
    // Cursor-based pagination
    // before: id of the last bookmark in the previous page. 0 for the first page.

    let mut conn = connection::establish();

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
        .load::<Bookmark>(&mut conn)
        .expect("Error search bookmarks")
}

pub fn create_bookmark(new_bookmark: NewBookmark) -> Bookmark {
    let mut conn = connection::establish();

    diesel::insert_into(bookmarks::table)
        .values(&new_bookmark)
        .returning(Bookmark::as_returning())
        .get_result(&mut conn)
        .expect("Error saving new bookmark")
}

pub fn delete_bookmarks(ids: Vec<i32>) -> usize {
    use diesel::{dsl::now, ExpressionMethods};

    use super::schema::bookmarks::{dsl::*, table};

    let mut conn = connection::establish();

    diesel::update(table)
        .filter(id.eq_any(ids).and(deleted_at.is_null()))
        .set(deleted_at.eq(now))
        .execute(&mut conn)
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
    use tracing::info;

    fn clean_bookmarks() {
        diesel::delete(bookmarks::table)
            .execute(&mut connection::establish())
            .expect("Error deleting bookmarks");
    }

    #[test]
    fn create_new_bookmark() {
        let new_bookmark = NewBookmark {
            title: "test".to_string(),
            url: "https://example.com".to_string(),
        };

        let m = create_bookmark(new_bookmark);

        info!("{:?}", m);
        assert!(m.id > 0);
    }

    #[test]
    fn title_search_bookmark() {
        create_new_bookmark();

        let mut conn = connection::establish();

        let results = bookmarks::table
            .filter(bookmarks::dsl::title.like("%test%"))
            .order_by(bookmarks::dsl::created_at.desc())
            .load::<Bookmark>(&mut conn)
            .expect("Error loading bookmarks");

        assert!(results.len() > 0);
        info!("{:?}", results[0]);
    }

    #[test]
    pub fn delete_a_bookmark() {
        let new_bookmark = NewBookmark {
            title: "test".to_string(),
            url: "https://example.com".to_string(),
        };
        let m = create_bookmark(new_bookmark);
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let count = delete_bookmarks(vec![m.id]);
        assert!(count == 1);

        let m = Bookmark::get(m.id).unwrap();
        assert!(m.deleted_at.is_some());
    }

    #[test]
    #[serial(search_bookmarks)]
    pub fn search_bookmarks_with_pagination() {
        clean_bookmarks();

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
            .execute(&mut connection::establish())
            .expect("Error saving new bookmarks");

        let results = search_bookmarks("", 0, 10);
        assert!(
            results.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks("Weather", 0, 10);
        assert!(
            results.len() == 3,
            "Expected 3 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks("Weather", 0, 2);
        assert!(
            results.len() == 2,
            "Expected 2 bookmarks, got {}",
            results.len()
        );

        let results = search_bookmarks("Weather", results[1].id, 2);
        assert!(
            results.len() == 1,
            "Expected 1 bookmarks, got {}",
            results.len()
        );
    }

    #[test]
    #[serial(unsearchable_deleted_bookmark)]
    fn unsearchable_deleted_bookmark() {
        let title = "invisible";
        let new_bookmark = NewBookmark {
            title: title.to_string(),
            url: "https://example.com".to_string(),
        };
        let m = create_bookmark(new_bookmark);
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let result = search_bookmarks(title, 0, 1);
        assert!(result.len() == 1);

        let count = delete_bookmarks(vec![m.id]);
        assert!(count == 1);

        let result = search_bookmarks(title, 0, 1);
        assert!(result.len() == 0);
    }
}
