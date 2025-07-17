use serde::{Deserialize, Serialize};

// API Response Types
#[derive(Deserialize, Serialize, Debug, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateFolder {
    pub path: String,
}

// Database Model Types
#[cfg(feature = "diesel")]
pub mod db {
    use diesel::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Queryable, Selectable, Identifiable, Debug, Clone, Deserialize, Serialize)]
    #[diesel(table_name = crate::folders)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    #[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
    pub struct Folder {
        pub id: i32,
        pub path: String,
        #[cfg_attr(feature = "utoipa", schema(format = DateTime, value_type=String))]
        #[serde(with = "time::serde::rfc3339")]
        pub created_at: time::OffsetDateTime,
        #[cfg_attr(feature = "utoipa", schema(format = DateTime, value_type=String))]
        #[serde(with = "time::serde::rfc3339")]
        pub updated_at: time::OffsetDateTime,
    }

    #[derive(Insertable, Debug, Clone)]
    #[diesel(table_name = crate::folders)]
    pub struct NewFolder<'a> {
        pub path: &'a str,
    }
}
