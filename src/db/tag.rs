use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use super::bookmark::{self, Bookmark};
use super::schema::{bookmarks_tags, tags};

#[derive(Queryable, Selectable, Identifiable, Debug, Deserialize, Serialize)]
#[diesel(table_name = tags)]
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
#[diesel(belongs_to(Bookmark))]
#[diesel(belongs_to(Tag))]
#[diesel(table_name = bookmarks_tags)]
#[diesel(primary_key(bookmark_id, tag_id))]
pub struct BookmarkTag {
    pub bookmark_id: i32,
    pub tag_id: i32,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = tags)]
pub struct NewTag {
    pub name: String,
}

async fn get_tags(conn: &mut Connection, tags: Vec<&str>) -> Vec<Tag> {
    tags::table
        .filter(tags::name.eq_any(tags))
        .load(conn)
        .await
        .expect("Error loading tags")
}

async fn get_or_create_tags(conn: &mut Connection, tags: Vec<&str>) -> Vec<Tag> {
    use diesel::{dsl::now, ExpressionMethods};
    let tags = tags
        .iter()
        .map(|name| NewTag {
            name: name.to_string(),
        })
        .collect::<Vec<_>>();

    diesel::insert_into(tags::table)
        .values(&tags)
        .on_conflict(tags::name)
        .do_update()
        .set(tags::updated_at.eq(now))
        .returning(Tag::as_returning())
        .get_results(conn)
        .await
        .expect("Error creating tags")
}

async fn update_bookmark_tags(conn: &mut Connection, bookmark: &Bookmark, tags: Vec<&str>) {
    let tags = get_or_create_tags(conn, tags).await;
    let bookmark_tags = tags
        .into_iter()
        .map(|tag| BookmarkTag {
            bookmark_id: bookmark.id,
            tag_id: tag.id,
        })
        .collect::<Vec<_>>();

    diesel::delete(bookmarks_tags::table.filter(bookmarks_tags::bookmark_id.eq(bookmark.id)))
        .execute(conn)
        .await
        .expect("Error deleting bookmark tags");

    diesel::insert_into(bookmarks_tags::table)
        .values(&bookmark_tags)
        .execute(conn)
        .await
        .expect("Error updating bookmark tags");
}

async fn get_tags_per_bookmark(
    conn: &mut Connection,
    bookmarks: Vec<Bookmark>,
) -> Vec<(Bookmark, Vec<Tag>)> {
    let tags = BookmarkTag::belonging_to(&bookmarks)
        .inner_join(tags::table)
        .select((BookmarkTag::as_select(), Tag::as_select()))
        .load(conn)
        .await
        .expect("Error loading tags");
    tags.grouped_by(&bookmarks)
        .into_iter()
        .zip(bookmarks)
        .map(|(tags, bookmark)| (bookmark, tags.into_iter().map(|(_, tag)| tag).collect()))
        .collect()
}

#[cfg(test)]
mod tests {
    use bookmark::tests::rand_bookmark;

    use crate::utils::rand::rand_str;

    use super::super::connection;
    use super::*;

    #[tokio::test]
    async fn test_get_or_create_tags() {
        let mut conn = connection::establish_async().await;

        let tags = vec![rand_str(4), rand_str(4)];
        let rv = get_tags(&mut conn, tags.clone().iter().map(|s| s.as_str()).collect()).await;
        assert_eq!(rv.len(), 0);

        let created_tags = get_or_create_tags(&mut conn, vec!["tag1", "tag2"]).await;
        assert_eq!(created_tags.len(), 2);

        let got_tags = get_or_create_tags(&mut conn, vec!["tag1", "tag2"]).await;
        assert_eq!(got_tags.len(), 2);

        assert_eq!(created_tags[0].id, got_tags[0].id);
        assert_eq!(created_tags[0].name, got_tags[0].name);
        assert_eq!(created_tags[1].id, got_tags[1].id);
        assert_eq!(created_tags[1].name, got_tags[1].name);
    }

    #[tokio::test]
    async fn test_update_bookmark_tags() {
        let mut conn = connection::establish_async().await;
        let new_bookmark = bookmark::create_bookmark(&mut conn, rand_bookmark()).await;

        update_bookmark_tags(&mut conn, &new_bookmark, vec!["tag1", "tag2"]).await;
        let bookmarks_tags = get_tags_per_bookmark(&mut conn, vec![new_bookmark.clone()]).await;

        assert_eq!(bookmarks_tags.len(), 1);
        let (bookmark, tags) = &bookmarks_tags[0];
        assert_eq!(bookmark.id, new_bookmark.id);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "tag1");
        assert_eq!(tags[1].name, "tag2");
    }
}
