// Database Model Types
#[cfg(feature = "diesel")]
pub mod db {
    use diesel::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Queryable, Selectable, Identifiable, Debug, Deserialize, Serialize)]
    #[diesel(table_name = crate::tags)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    pub struct Tag {
        pub id: i32,
        pub name: String,
        #[serde(with = "time::serde::rfc3339")]
        pub created_at: time::OffsetDateTime,
        #[serde(with = "time::serde::rfc3339")]
        pub updated_at: time::OffsetDateTime,
    }

    #[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug)]
    #[diesel(belongs_to(crate::bookmark::db::Bookmark))]
    #[diesel(belongs_to(Tag))]
    #[diesel(table_name = crate::bookmarks_tags)]
    #[diesel(primary_key(bookmark_id, tag_id))]
    pub struct BookmarkTag {
        pub bookmark_id: i32,
        pub tag_id: i32,
    }

    #[derive(Insertable, Debug, Clone)]
    #[diesel(table_name = crate::tags)]
    pub struct NewTag {
        pub name: String,
    }

    #[derive(AsChangeset, Deserialize, Serialize, Debug)]
    #[diesel(table_name = crate::tags)]
    pub struct ModifyTag {
        pub name: Option<String>,
    }
}
