// @generated automatically by Diesel CLI.

diesel::table! {
    bookmarks (id) {
        id -> Int4,
        title -> Varchar,
        url -> Varchar,
        created_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    bookmarks_tags (bookmark_id, tag_id) {
        bookmark_id -> Int4,
        tag_id -> Int4,
    }
}

diesel::table! {
    tags (id) {
        id -> Int4,
        name -> Varchar,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(bookmarks_tags -> bookmarks (bookmark_id));
diesel::joinable!(bookmarks_tags -> tags (tag_id));

diesel::allow_tables_to_appear_in_same_query!(bookmarks, bookmarks_tags, tags,);
