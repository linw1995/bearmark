use serde::{Deserialize, Serialize};

// API Response Types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Bookmark {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    #[cfg_attr(feature = "utoipa", schema(format = DateTime, value_type=String))]
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[cfg_attr(feature = "utoipa", schema(format = DateTime, value_type=String, nullable))]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<time::OffsetDateTime>,
    #[cfg_attr(feature = "utoipa", schema(format = DateTime, value_type=String))]
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateBookmark {
    pub title: String,
    pub url: String,
    pub folder_id: Option<i32>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ModifyBookmark {
    pub title: Option<String>,
    pub url: Option<String>,
    pub tags: Option<Vec<String>>,
}

// Database Model Types
#[cfg(feature = "diesel")]
pub mod db {
    use diesel::prelude::*;
    use serde::{Deserialize, Serialize};

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
    #[diesel(table_name = crate::bookmarks)]
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
    #[diesel(table_name = crate::bookmarks)]
    pub struct NewBookmark {
        pub title: String,
        pub url: String,
    }

    #[derive(AsChangeset, Deserialize, Serialize, Debug, Clone)]
    #[diesel(table_name = crate::bookmarks)]
    pub struct ModifyBookmark {
        pub title: Option<String>,
        pub url: Option<String>,
    }
}
