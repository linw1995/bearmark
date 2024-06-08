// @generated automatically by Diesel CLI.

diesel::table! {
    bookmarks (id) {
        id -> Int4,
        title -> Varchar,
        url -> Varchar,
        created_at -> Timestamptz,
    }
}
