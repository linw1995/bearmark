use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};

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
}
