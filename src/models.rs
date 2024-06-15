use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Debug, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::bookmarks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Bookmark {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub created_at: time::OffsetDateTime,
}

#[derive(Insertable, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::bookmarks)]
pub struct NewBookmark {
    pub title: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::establish_connection;
    use tracing::info;

    #[test]
    fn test_create_bookmark() {
        let mut conn = establish_connection();

        let new_bookmark = NewBookmark {
            title: "test".to_string(),
            url: "https://example.com".to_string(),
        };

        let m = diesel::insert_into(crate::schema::bookmarks::table)
            .values(&new_bookmark)
            .returning(Bookmark::as_returning())
            .get_result(&mut conn)
            .expect("Error saving new bookmark");

        info!("{:?}", m);
        assert!(m.id > 0);
    }
}
