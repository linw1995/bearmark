use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};

use super::connection;
use super::schema::bookmarks;

#[derive(Queryable, Selectable, Debug, Deserialize, Serialize)]
#[diesel(table_name = bookmarks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Bookmark {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub created_at: time::OffsetDateTime,
}

pub fn search_bookmarks(title: &str, before: i32, limit: i64) -> Vec<Bookmark> {
    // Cursor-based pagination
    // before: id of the last bookmark in the previous page. 0 for the first page.

    let mut conn = connection::establish();

    let mut query = bookmarks::table
        .filter(bookmarks::dsl::title.like(format!("%{}%", title)))
        .into_boxed();

    if before > 0 {
        query = query.filter(bookmarks::dsl::id.lt(before))
    }

    query
        .order_by(bookmarks::dsl::id.desc())
        .limit(limit)
        .load::<Bookmark>(&mut conn)
        .expect("Error search bookmarks")
}

#[derive(Insertable, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = bookmarks)]
pub struct NewBookmark {
    pub title: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::super::connection;
    use super::*;
    use tracing::info;

    fn clean_bookmarks() {
        diesel::delete(bookmarks::table)
            .execute(&mut connection::establish())
            .expect("Error deleting bookmarks");
    }

    #[test]
    fn create_bookmark() {
        let mut conn = connection::establish();

        let new_bookmark = NewBookmark {
            title: "test".to_string(),
            url: "https://example.com".to_string(),
        };

        let m = diesel::insert_into(bookmarks::table)
            .values(&new_bookmark)
            .returning(Bookmark::as_returning())
            .get_result(&mut conn)
            .expect("Error saving new bookmark");

        info!("{:?}", m);
        assert!(m.id > 0);
    }

    #[test]
    fn title_search_bookmark() {
        create_bookmark();

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
    fn search_bookmarks_with_pagination() {
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

        let results = search_bookmarks("Weather", 0, 10);
        assert!(results.len() == 3);

        let results = search_bookmarks("Weather", 0, 2);
        assert!(results.len() == 2);

        let results = search_bookmarks("Weather", results[1].id, 2);
        assert!(results.len() == 1);
    }
}
