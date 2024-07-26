use diesel::prelude::*;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};
use tracing::debug;

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

#[derive(AsChangeset, Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = tags)]
pub struct ModifyTag {
    pub name: Option<String>,
}

pub async fn get_tags(conn: &mut Connection, tags: &Vec<String>) -> Vec<Tag> {
    tags::table
        .filter(tags::name.eq_any(tags))
        .load(conn)
        .await
        .expect("Error loading tags")
}

pub async fn get_or_create_tags(conn: &mut Connection, tags: &[String]) -> Vec<Tag> {
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

pub async fn update_bookmark_tags(conn: &mut Connection, bookmark: &Bookmark, tags: &[String]) {
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
        .on_conflict_do_nothing()
        .execute(conn)
        .await
        .expect("Error updating bookmark tags");
}

pub async fn get_tags_per_bookmark(
    conn: &mut Connection,
    bookmarks: Vec<Bookmark>,
) -> Vec<(Bookmark, Vec<Tag>)> {
    let tags = BookmarkTag::belonging_to(&bookmarks)
        .inner_join(tags::table)
        .select((BookmarkTag::as_select(), Tag::as_select()))
        .order_by((
            bookmarks_tags::dsl::bookmark_id.desc(),
            tags::dsl::name.asc(),
        ))
        .load(conn)
        .await
        .expect("Error loading tags");
    tags.grouped_by(&bookmarks)
        .into_iter()
        .zip(bookmarks)
        .map(|(tags, bookmark)| (bookmark, tags.into_iter().map(|(_, tag)| tag).collect()))
        .collect()
}

pub async fn search_tags(
    conn: &mut Connection,
    keywords: &Vec<&str>,
    before: i32,
    limit: i64,
) -> Vec<Tag> {
    let mut query = tags::table.select(Tag::as_select()).into_boxed();

    for keyword in keywords {
        query = query.filter(tags::dsl::name.ilike(format!("%{}%", keyword)));
    }

    if before > 0 {
        query = query.filter(tags::dsl::id.lt(before))
    }

    query
        .order_by(tags::dsl::id.desc())
        .limit(limit)
        .load(conn)
        .await
        .expect("Error loading tags")
}

pub async fn search_bookmarks(
    conn: &mut Connection,
    keywords: &Vec<&str>,
    tags: &Vec<&str>,
    before: i32,
    limit: i64,
) -> Vec<(Bookmark, Vec<Tag>)> {
    let bs = if tags.is_empty() {
        bookmark::search_bookmarks(conn, keywords, before, limit).await
    } else {
        use super::schema::{self, bookmarks, bookmarks_tags};

        let mut query = bookmarks_tags::table
            .inner_join(bookmarks::table)
            .inner_join(schema::tags::table)
            .select(Bookmark::as_select())
            .distinct_on(bookmarks::id)
            .filter(bookmarks::dsl::deleted_at.is_null())
            .into_boxed();

        for tag in tags {
            if tag.is_empty() {
                continue;
            }
            query = query.filter(tags::dsl::name.ilike(format!("%{}%", tag)));
        }

        for keyword in keywords {
            if keyword.is_empty() {
                continue;
            }
            query = query.filter(
                bookmarks::dsl::title
                    .ilike(format!("%{}%", keyword))
                    .or(bookmarks::dsl::url.ilike(format!("%{}%", keyword))),
            )
        }

        if before > 0 {
            query = query.filter(bookmarks::dsl::id.lt(before))
        }

        query
            .order_by(bookmarks::dsl::id.desc())
            .limit(limit)
            .load::<Bookmark>(conn)
            .await
            .expect("Error loading bookmarks")
    };

    debug!(?bs, "bookmarks without tags");

    // Group bookmark and tags
    let bs_ts = get_tags_per_bookmark(conn, bs).await;
    debug!(?bs_ts, "bookmarks with tags");
    bs_ts
}

pub async fn delete_tags(conn: &mut Connection, ids: Vec<i32>) -> usize {
    if ids.is_empty() {
        return 0;
    }
    diesel::delete(tags::table)
        .filter(tags::dsl::id.eq_any(ids))
        .execute(conn)
        .await
        .expect("Error deleting tags")
}

pub async fn update_tag(conn: &mut Connection, id: i32, modified: ModifyTag) -> Option<Tag> {
    use diesel::{dsl::now, ExpressionMethods};

    diesel::update(tags::table.find(id))
        .set((&modified, tags::updated_at.eq(now)))
        .returning(Tag::as_returning())
        .get_result(conn)
        .await
        .optional()
        .expect("Error updating tag")
}

#[cfg(test)]
pub mod tests {
    use super::super::bookmark::{self, tests::rand_bookmark};
    use super::super::connection;
    use super::*;
    use crate::db::bookmark::NewBookmark;
    use crate::db::schema::bookmarks;
    use crate::utils::rand::rand_str;

    use itertools::Itertools;
    use tracing::info;

    #[tokio::test]
    async fn test_get_or_create_tags() {
        let mut conn = connection::establish().await;

        let tag_names = vec![rand_str(4), rand_str(4)];
        let rv = get_tags(&mut conn, &tag_names).await;
        info!(?rv, "tags not found");
        assert_eq!(rv.len(), 0);

        let created_tags = get_or_create_tags(&mut conn, &tag_names).await;
        info!(?created_tags, "created tags");
        assert_eq!(created_tags.len(), 2);

        let got_tags = get_or_create_tags(&mut conn, &tag_names).await;
        info!(?got_tags, "got tags");
        assert_eq!(got_tags.len(), 2);

        assert_eq!(created_tags[0].id, got_tags[0].id);
        assert_eq!(created_tags[0].name, got_tags[0].name);
        assert_eq!(created_tags[1].id, got_tags[1].id);
        assert_eq!(created_tags[1].name, got_tags[1].name);
    }

    #[tokio::test]
    async fn test_update_bookmark_tags() {
        let mut conn = connection::establish().await;

        let new_bookmark = bookmark::create_bookmark(&mut conn, rand_bookmark()).await;
        info!(?new_bookmark, "created bookmark");

        let bookmarks_tags = get_tags_per_bookmark(&mut conn, vec![new_bookmark.clone()]).await;
        assert_eq!(bookmarks_tags.len(), 1);
        let (bookmark, tags) = &bookmarks_tags[0];
        info!(?bookmark, ?tags, "bookmark has no tags");
        assert_eq!(bookmark.id, new_bookmark.id);
        assert!(tags.is_empty());

        let tag_names = vec![rand_str(4), rand_str(4)]
            .iter()
            .sorted()
            .map(|i| i.to_string())
            .collect_vec();
        update_bookmark_tags(&mut conn, &new_bookmark, &tag_names).await;
        info!(?tag_names, "updated bookmark tags");

        let bookmarks_tags = get_tags_per_bookmark(&mut conn, vec![new_bookmark.clone()]).await;
        assert_eq!(bookmarks_tags.len(), 1);
        let (bookmark, tags) = &bookmarks_tags[0];
        info!(?bookmark, ?tags, "bookmark has tags");
        assert_eq!(bookmark.id, new_bookmark.id);
        assert_eq!(tags.len(), 2);
        assert_eq!(
            tags.into_iter()
                .map(|t| t.name.clone())
                .sorted()
                .collect_vec(),
            tag_names
        );
    }

    pub async fn setup_searchable_bookmarks(conn: &mut Connection) {
        let values = vec![
            (
                NewBookmark {
                    title: "Weather".to_string(),
                    url: "https://weather.com".to_string(),
                },
                vec!["weather", "forecast"],
            ),
            (
                NewBookmark {
                    title: "News".to_string(),
                    url: "https://news.com".to_string(),
                },
                vec!["news", "world"],
            ),
            (
                NewBookmark {
                    title: "Sports".to_string(),
                    url: "https://sports.com".to_string(),
                },
                vec!["sports", "football"],
            ),
            (
                NewBookmark {
                    title: "Tech".to_string(),
                    url: "https://tech.com".to_string(),
                },
                vec!["tech", "gadgets"],
            ),
            (
                NewBookmark {
                    title: "Weather Global".to_string(),
                    url: "https://example.com".to_string(),
                },
                vec!["weather", "global"],
            ),
            (
                NewBookmark {
                    title: "Weather West".to_string(),
                    url: "https://example.com".to_string(),
                },
                vec!["weather", "west"],
            ),
        ];

        // delete bookmarks with same title
        let titles = values
            .iter()
            .map(|(b, _)| b.title.clone())
            .collect::<Vec<_>>();
        diesel::delete(bookmarks::table)
            .filter(bookmarks::title.eq_any(titles))
            .execute(conn)
            .await
            .expect("Error deleting bookmarks");

        for (bookmark, tags) in values {
            let bookmark = bookmark::create_bookmark(conn, bookmark).await;
            let tags = tags.iter().map(|t| t.to_string()).collect_vec();
            update_bookmark_tags(conn, &bookmark, &tags).await;
        }
    }

    #[tokio::test]
    #[file_serial] // For allowing remove data of table in test
    async fn test_search_bookmarks() {
        let mut conn = connection::establish().await;
        setup_searchable_bookmarks(&mut conn).await;

        let bookmarks = search_bookmarks(&mut conn, &vec!["Weather"], &vec![], 0, 10).await;
        info!(?bookmarks, "searched bookmarks");
        assert_eq!(bookmarks.len(), 3);

        let bookmarks = search_bookmarks(&mut conn, &vec!["Weather"], &vec!["global"], 0, 10).await;
        info!(?bookmarks, "searched bookmarks with tags");
        assert_eq!(bookmarks.len(), 1);

        let bookmarks = search_bookmarks(&mut conn, &vec!["Weather"], &vec!["west"], 0, 10).await;
        info!(?bookmarks, "searched bookmarks with tags");
        assert_eq!(bookmarks.len(), 1);

        let bookmarks =
            search_bookmarks(&mut conn, &vec!["Weather"], &vec!["global", "west"], 0, 10).await;
        info!(?bookmarks, "searched bookmarks with tags");
        assert_eq!(bookmarks.len(), 0);

        let bookmarks = search_bookmarks(&mut conn, &vec![], &vec!["weather"], 0, 10).await;
        info!(?bookmarks, "searched bookmarks with tags");
        assert_eq!(bookmarks.len(), 3);

        let bookmarks = search_bookmarks(&mut conn, &vec![], &vec!["weather"], 0, 1).await;
        info!(?bookmarks, "searched bookmarks with tags");
        assert_eq!(bookmarks.len(), 1);

        let bookmarks =
            search_bookmarks(&mut conn, &vec![], &vec!["weather"], bookmarks[0].0.id, 3).await;
        info!(?bookmarks, "searched bookmarks with tags");
        assert_eq!(bookmarks.len(), 2);
    }

    #[tokio::test]
    async fn test_search_tags() {
        let mut conn = connection::establish().await;
        let tags = vec![
            "weather#1",
            "weather#2",
            "weather#3",
            "global#1",
            "global#2",
        ]
        .into_iter()
        .map(|t| t.to_string())
        .collect_vec();

        get_or_create_tags(&mut conn, &tags).await;

        let tags = search_tags(&mut conn, &vec!["weather#"], 0, 10).await;
        info!(?tags, "searched tags");
        assert_eq!(tags.len(), 3);

        let tags = search_tags(&mut conn, &vec!["global#"], 0, 4).await;
        info!(?tags, "searched tags");
        assert_eq!(tags.len(), 2);
    }
}
