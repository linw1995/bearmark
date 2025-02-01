use diesel::expression::BoxableExpression;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::{AsyncPgConnection as Connection, RunQueryDsl};
use tracing::{debug, warn};

use super::bookmark::Bookmark;
use super::folder::Folder;
use super::tag::Tag;
use crate::db::schema;
use crate::utils::{BearQLError, CommonError};

fn parse_query<'a>(
    raw: &str,
    bump: &'a bumpalo::Bump,
) -> Result<bearmark_ql::Query<'a>, BearQLError> {
    use bearmark_ql::{Parser, Query};
    debug!(?raw, "parsing query");
    let rv = Parser::<Query>::parse_with(raw, bump).map_err(|e| {
        warn!(?raw, ?e, "failed to parse query");
        BearQLError::SyntaxError {
            msg: "failed to parse query".to_string(),
            ql: raw.to_string(),
            err_msg: format!("{:?}", e),
        }
    })?;
    debug!(?rv, "parsed query");
    Ok(rv)
}

fn join_folder_path(cwd: &str, p: &str) -> String {
    const PATH_SEP: char = '/';
    if p.starts_with(PATH_SEP) {
        p.to_string()
    } else {
        cwd.trim_end_matches(PATH_SEP)
            .split(PATH_SEP)
            .chain(p.split(PATH_SEP))
            .filter(|&x| x != ".")
            .collect::<Vec<_>>()
            .join(&PATH_SEP.to_string())
    }
}

fn find_bookmarks_in_path(
    p: &str,
) -> Result<Box<dyn BoxableExpression<schema::bookmarks::table, Pg, SqlType = Bool>>, CommonError> {
    use super::schema::{bookmarks, folders};

    let without_descendants = p.ends_with("//");
    let p = p.trim_end_matches('/').to_string(); // remove trailing slashes

    if p.is_empty() {
        return Err(CommonError::InvalidCWD);
    }

    let expression: Box<dyn BoxableExpression<_, _, SqlType = Bool>> = if without_descendants {
        // special syntax. search bookmarks in the folder only
        Box::new(folders::dsl::path.eq(p))
    } else {
        // search bookmarks in the folder and its descendants
        Box::new(
            folders::dsl::path
                .like(format!("{}/%", p))
                .or(folders::dsl::path.eq(p)),
        )
    };
    let folder_ids = folders::table.filter(expression).select(
        folders::id.nullable(), // .nullable() is a dirty patch to make check pass, no side effects
    );
    let condition = bookmarks::dsl::folder_id
        .eq_any(folder_ids)
        .assume_not_null(); // .assume_not_null() is a dirty patch to make check pass, no side effects
    Ok(Box::new(condition))
}

fn find_bookmarks(
    query: &bearmark_ql::Query,
    cwd: &str,
    cwd_overwrited: &mut bool,
) -> Result<Box<dyn BoxableExpression<schema::bookmarks::table, Pg, SqlType = Bool>>, CommonError> {
    use super::schema::{bookmarks, bookmarks_tags, tags};
    use bearmark_ql::Query::*;

    Ok(match query {
        Or(a, b) => Box::new(find_bookmarks(a, cwd, cwd_overwrited)?.or(find_bookmarks(
            b,
            cwd,
            cwd_overwrited,
        )?)),
        And(a, b) => Box::new(find_bookmarks(a, cwd, cwd_overwrited)?.and(find_bookmarks(
            b,
            cwd,
            cwd_overwrited,
        )?)),
        Parenthesized(a) => find_bookmarks(a, cwd, cwd_overwrited)?,
        Path(p) => {
            let target = p.to_string();
            let path = join_folder_path(cwd, &target);
            *cwd_overwrited = true;
            debug!(?path, ?cwd, ?target, "searching in path");
            if path == "/" {
                Box::new(bookmarks::dsl::id.eq(bookmarks::dsl::id)) // always true, no side effects
            } else if path == "//" {
                Box::new(bookmarks::dsl::folder_id.is_null()) // special syntax. search bookmarks which are not in any folder
            } else {
                find_bookmarks_in_path(&path)?
            }
        }
        Tag(t) => {
            let t = t.to_string();
            let t = t.trim_start_matches('#').trim().to_string();
            if t.is_empty() {
                return Err(CommonError::BearQL(BearQLError::EmptyTag));
            }
            let bookmarks = diesel::alias!(bookmarks as bm);
            Box::new(
                bookmarks::dsl::id.eq_any(
                    bookmarks
                        .inner_join(bookmarks_tags::table)
                        .filter(
                            bookmarks_tags::dsl::tag_id
                                .eq_any(tags::table.filter(tags::dsl::name.eq(t)).select(tags::id)),
                        )
                        .select(bookmarks.fields(bookmarks::id))
                        .distinct(),
                ),
            )
        }
        Keyword(k) => {
            let k = k.to_string();
            let k = k.trim().to_string();
            if k.is_empty() {
                return Err(CommonError::BearQL(BearQLError::EmptyKeyword));
            }
            Box::new(
                bookmarks::dsl::title
                    .ilike(format!("%{}%", k))
                    .or(bookmarks::dsl::url.ilike(format!("%{}%", k))),
            )
        }
    })
}

/// Search bookmarks by paths, keywords, and tags.
pub async fn search_bookmarks(
    conn: &mut Connection,
    query: Option<&str>,
    cwd: Option<&str>,
    before: i32,
    limit: i64,
) -> Result<Vec<(Bookmark, Option<Folder>, Vec<Tag>)>, CommonError> {
    use super::schema::bookmarks;

    let mut builder = bookmarks::table
        .select(Bookmark::as_select())
        .distinct_on(bookmarks::id)
        .filter(bookmarks::dsl::deleted_at.is_null())
        .into_boxed();

    let mut cwd_overwrited = false;
    if let Some(query) = query {
        let cwd = cwd.unwrap_or("/");
        let bump = bumpalo::Bump::new();
        let query = parse_query(query, &bump)?;
        builder = builder.filter(find_bookmarks(&query, cwd, &mut cwd_overwrited)?)
    }
    if !cwd_overwrited {
        if let Some(cwd) = cwd {
            if cwd != "/" {
                let expression = if cwd == "//" {
                    Box::new(bookmarks::dsl::folder_id.is_null()) // special syntax. search bookmarks which are not in any folder
                } else {
                    find_bookmarks_in_path(cwd)?
                };
                builder = builder.filter(expression);
            }
        }
    }

    if before > 0 {
        builder = builder.filter(bookmarks::dsl::id.lt(before));
    }

    let lst = builder
        .order_by(bookmarks::id.desc())
        .limit(limit)
        .load::<Bookmark>(conn)
        .await
        .expect("Error loading bookmarks");
    Ok(if lst.is_empty() {
        vec![]
    } else {
        get_bookmark_details(conn, lst).await
    })
}

pub async fn get_bookmark_details(
    conn: &mut Connection,
    bookmarks: Vec<Bookmark>,
) -> Vec<(Bookmark, Option<Folder>, Vec<Tag>)> {
    use std::collections::HashMap;

    use super::schema::{bookmarks_tags, folders, tags};
    use super::tag::BookmarkTag;

    let folder_ids = bookmarks
        .iter()
        .filter_map(|b| b.folder_id)
        .collect::<Vec<_>>();

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

    let folder_map = if folder_ids.is_empty() {
        HashMap::new()
    } else {
        let folders = folders::table
            .select(Folder::as_select())
            .filter(folders::id.eq_any(folder_ids))
            .load::<Folder>(conn)
            .await
            .expect("Error loading folders");
        HashMap::from_iter(folders.into_iter().map(|f| (f.id, f)))
    };
    tags.grouped_by(&bookmarks)
        .into_iter()
        .zip(bookmarks)
        .map(|(tags, bookmark)| {
            let folder = bookmark
                .folder_id
                .and_then(|id| folder_map.get(&id).cloned());
            (
                bookmark,
                folder,
                tags.into_iter().map(|(_, tag)| tag).collect(),
            )
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::db::bookmark::test::{create_rand_bookmark, rand_bookmark};
    use crate::db::bookmark::NewBookmark;
    use crate::db::bookmark::{create_bookmark, delete_bookmarks};
    use crate::db::connection;
    use crate::db::folder::{create_folder, move_bookmarks};
    use crate::db::schema::bookmarks;
    use crate::db::tag::update_bookmark_tags;
    use crate::utils::rand::rand_str;
    use crate::utils::DatabaseError;

    use itertools::Itertools;
    use tracing::{debug, info};

    #[test]
    fn test_join_path() {
        for (cwd, p, expect) in &[
            ("/", "./", "/"),
            ("/", "/", "/"),
            ("/", "./a", "/a"),
            ("/b", "/a", "/a"),
            ("/", "./a/", "/a/"),
            ("/b", "/a/", "/a/"),
            ("/", "./a//", "/a//"),
            ("/b", "/a//", "/a//"),
            ("/", "/a/b", "/a/b"),
            ("/", "./a/b", "/a/b"),
            ("/a", "./b/c", "/a/b/c"),
            ("/a", "./b/c/", "/a/b/c/"),
        ] {
            debug!(?cwd, ?p, ?expect, "testing join path");
            let rv = join_folder_path(cwd, p);
            assert_eq!(rv, *expect);
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Query {
        Or(Box<Self>, Box<Self>),
        And(Box<Self>, Box<Self>),
        Parenthesized(Box<Self>),

        Path(String),
        Tag(String),
        Keyword(String),
    }

    fn simplify_query(q: &bearmark_ql::Query) -> Query {
        use Query::*;
        match q {
            bearmark_ql::Query::Or(a, b) => {
                Or(Box::new(simplify_query(a)), Box::new(simplify_query(b)))
            }
            bearmark_ql::Query::And(a, b) => {
                And(Box::new(simplify_query(a)), Box::new(simplify_query(b)))
            }
            bearmark_ql::Query::Parenthesized(a) => Parenthesized(Box::new(simplify_query(a))),
            bearmark_ql::Query::Path(p) => Path(p.to_string()),
            bearmark_ql::Query::Tag(t) => Tag(t.to_string().trim_start_matches('#').to_string()),
            bearmark_ql::Query::Keyword(k) => Keyword(k.to_string()),
        }
    }

    #[test]
    fn test_parse_query() {
        use Query::*;

        let bump = bumpalo::Bump::new();
        for (raw, expect) in &[
            ("rust", Keyword("rust".into())),
            ("#rust", Tag("rust".into())),
            ("/", Path("/".into())),
            ("./", Path("./".into())),
            ("./langs/rust", Path("./langs/rust".into())),
            ("./langs/rust//", Path("./langs/rust//".into())),
            ("//", Path("//".into())),
            (".//", Path(".//".into())),
            ("/blog/", Path("/blog/".into())),
            (
                "title #rust",
                And(
                    Box::new(Keyword("title".into())),
                    Box::new(Tag("rust".into())),
                ),
            ),
            (
                "rust | langs go",
                Or(
                    Box::new(Keyword("rust".into())),
                    Box::new(And(
                        Box::new(Keyword("langs".into())),
                        Box::new(Keyword("go".into())),
                    )),
                ),
            ),
            (
                "(rust | langs) go",
                And(
                    Box::new(Parenthesized(Box::new(Or(
                        Box::new(Keyword("rust".into())),
                        Box::new(Keyword("langs".into())),
                    )))),
                    Box::new(Keyword("go".into())),
                ),
            ),
            (
                "(#rust | #go) test",
                And(
                    Box::new(Parenthesized(Box::new(Or(
                        Box::new(Tag("rust".into())),
                        Box::new(Tag("go".into())),
                    )))),
                    Box::new(Keyword("test".into())),
                ),
            ),
        ] {
            let query = parse_query(raw, &bump).unwrap();
            let query = simplify_query(&query);
            info!(?raw, ?query, ?expect, "testing parse query");
            assert_eq!(query, *expect);
        }
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

        for (new, tags) in values {
            let bookmark = create_bookmark(conn, &new).await;
            let tags = tags.iter().map(|t| t.to_string()).collect_vec();
            update_bookmark_tags(conn, &bookmark, &tags).await;
        }
    }

    #[tokio::test]
    pub async fn search_bookmarks_with_pagination() {
        let mut conn = connection::establish().await;
        setup_searchable_bookmarks(&mut conn).await;

        let rv = search_bookmarks(&mut conn, None, None, 0, 10).await;
        info!(?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert!(
            rv.len() >= 5,
            "Expected more than 5 bookmarks, got {}",
            rv.len()
        );

        let rv = search_bookmarks(&mut conn, Some("Weather"), None, 0, 10).await;
        info!(?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert!(
            rv.len() >= 3,
            "Expected more than 3 bookmarks, got {}",
            rv.len()
        );

        let rv = search_bookmarks(&mut conn, Some("Weather"), None, 0, 2).await;
        info!(?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert!(rv.len() == 2, "Expected 2 bookmarks, got {}", rv.len());

        let rv = search_bookmarks(&mut conn, Some("Weather"), None, rv[1].0.id, 2).await;
        info!(?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert!(rv.len() == 1, "Expected 1 bookmarks, got {}", rv.len());
    }

    #[tokio::test]
    async fn unsearchable_deleted_bookmark() {
        let mut conn = connection::establish().await;
        let new = rand_bookmark();
        let title = new.title.clone();
        let m = create_bookmark(&mut conn, &new).await;
        info!(?m, "created");
        assert!(m.id > 0);
        assert!(m.deleted_at.is_none());

        let result = search_bookmarks(&mut conn, Some(&title), None, 0, 1).await;
        info!(?result, "searched");
        let result = result.unwrap();
        assert_eq!(result.len(), 1);

        let count = delete_bookmarks(&mut conn, vec![m.id]).await;
        assert_eq!(count, 1);

        let result = search_bookmarks(&mut conn, Some(&title), None, 0, 1).await;
        info!(?result, "searched");
        let result = result.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_search_bookmarks_with_tags() {
        let mut conn = connection::establish().await;
        setup_searchable_bookmarks(&mut conn).await;

        let rv = search_bookmarks(&mut conn, Some("Weather"), None, 0, 10).await;
        info!(?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 3);

        let rv = search_bookmarks(&mut conn, Some("Weather #global"), None, 0, 10).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 1);

        let rv = search_bookmarks(&mut conn, Some("Weather #west"), None, 0, 10).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 1);

        let rv = search_bookmarks(&mut conn, Some("#weather #global"), None, 0, 10).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 1);

        let rv = search_bookmarks(&mut conn, Some("Weather #west #global"), None, 0, 10).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 0);

        let rv = search_bookmarks(&mut conn, Some("#weather"), None, 0, 10).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 3);

        info!("search bookmarks with pagination, limit first");

        let rv = search_bookmarks(&mut conn, Some("#weather"), None, 0, 1).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 1);

        info!("search bookmarks with pagination, paginated by cursor");
        let rv = search_bookmarks(&mut conn, Some("#weather"), None, rv[0].0.id, 3).await;
        info!(?rv, "searched bookmarks with tag");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 2);
    }

    async fn setup_folders_and_bookmarks(
        conn: &mut Connection,
        folder_bookmarks_counter: Vec<(String, usize)>,
    ) -> Result<(), DatabaseError> {
        for (folder_name, bookmarks_count) in folder_bookmarks_counter {
            let folder = create_folder(conn, &folder_name).await?;
            let mut bookmark_ids = vec![];
            for _ in 0..bookmarks_count {
                let bookmark = create_rand_bookmark(conn).await;
                bookmark_ids.push(bookmark.id);
            }
            move_bookmarks(conn, folder.id, &bookmark_ids).await?;
        }
        Ok(())
    }

    macro_rules! assert_searched_bookmarks {
        ($query:expr, $cwd:expr, $expected_size:literal) => {
            let mut conn = connection::establish().await;
            let query: Option<&str> = $query;
            let cwd: Option<&str> = $cwd;
            let rv = search_bookmarks(&mut conn, query, cwd, 0, 100).await;
            info!(?query, ?cwd, ?rv, "searched bookmarks");
            let rv = rv.unwrap();
            assert_eq!(rv.len(), $expected_size);
        };
    }

    struct SetupFoldersAndBookmarksDefaultReturn {
        folder1_path: String,
        folder2_path: String,
        folder2_id: i32,
        folder3_name: String,
    }

    async fn setup_folders_and_bookmarks_default(
        conn: &mut Connection,
    ) -> SetupFoldersAndBookmarksDefaultReturn {
        let folder1_path = format!("/{}", rand_str(10));
        let folder2_path = format!("/{}", rand_str(10));
        let folder3_name = rand_str(10);
        let folder3_path = format!("{}/{}", folder1_path, folder3_name);

        setup_folders_and_bookmarks(
            conn,
            vec![
                (folder1_path.clone(), 10),
                (folder2_path.clone(), 1),
                (folder3_path.clone(), 1), // descendant of folder1
            ],
        )
        .await
        .unwrap();

        let folder2 = Folder::get_by_path(conn, &folder2_path).await.unwrap();

        SetupFoldersAndBookmarksDefaultReturn {
            folder1_path,
            folder2_path,
            folder2_id: folder2.id,
            folder3_name,
        }
    }

    #[tokio::test]
    async fn search_bookmarks_in_folders() {
        let mut conn = connection::establish().await;

        let SetupFoldersAndBookmarksDefaultReturn {
            folder1_path,
            folder2_path,
            ..
        } = setup_folders_and_bookmarks_default(&mut conn).await;

        info!("search bookmarks in folder1 and its descendants");
        assert_searched_bookmarks!(Some(&folder1_path), None, 11);

        info!("search bookmarks in folder2");
        assert_searched_bookmarks!(Some(&folder2_path), None, 1);

        let query = format!("{} | {}", folder1_path, folder2_path);
        info!(?query, "search bookmarks in folder1 and folder2");
        assert_searched_bookmarks!(Some(&query), None, 12);

        let query = format!("{}//", folder1_path);
        info!(?query, "search bookmarks in folder1 only");
        assert_searched_bookmarks!(Some(&query), None, 10);

        let query = format!("{}// | {}", folder1_path, folder2_path);
        info!(?query, "search bookmarks in folder1 only and folder2");
        assert_searched_bookmarks!(Some(&query), None, 11);
    }

    #[tokio::test]
    async fn search_bookmarks_in_folders_pagination() {
        let mut conn = connection::establish().await;

        let SetupFoldersAndBookmarksDefaultReturn {
            folder1_path,
            folder2_path,
            ..
        } = setup_folders_and_bookmarks_default(&mut conn).await;

        let query = format!("{} | {}", folder1_path, folder2_path);
        let rv = search_bookmarks(&mut conn, Some(&query), None, 0, 5).await;
        info!(?query, ?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 5);

        let rv = search_bookmarks(&mut conn, Some(&query), None, rv[4].0.id, 100).await;
        info!(?query, ?rv, "searched bookmarks");
        let rv = rv.unwrap();
        assert_eq!(rv.len(), 7);
    }

    #[tokio::test]
    async fn search_bookmarks_in_folders_after_normalizing() {
        let mut conn = connection::establish().await;

        let SetupFoldersAndBookmarksDefaultReturn { folder1_path, .. } =
            setup_folders_and_bookmarks_default(&mut conn).await;

        let query = format!("{}/", folder1_path);
        info!(
            ?query,
            "search bookmarks in folder1 and its descendants with single tailing slash"
        );
        assert_searched_bookmarks!(Some(&query), None, 11);
    }

    #[tokio::test]
    async fn search_bookmarks_with_invalid_cwd() {
        let mut conn = connection::establish().await;
        let rv = search_bookmarks(&mut conn, None, Some("///"), 0, 100).await;
        info!(?rv, "searched bookmarks");
        assert!(rv.is_err());
        let rv = rv.unwrap_err();
        assert!(matches!(rv, CommonError::InvalidCWD));
    }

    #[tokio::test]
    async fn search_bookmarks_in_root() {
        let mut conn = connection::establish().await;

        // delete all bookmarks
        diesel::delete(bookmarks::table)
            .execute(&mut conn)
            .await
            .expect("Error deleting bookmarks");

        let _ = create_rand_bookmark(&mut conn).await;

        let _ = setup_folders_and_bookmarks_default(&mut conn).await;

        info!("search bookmarks in root");
        assert_searched_bookmarks!(None, None, 13); // 11 + 1 + 1

        info!("search bookmarks in root explicitly");
        assert_searched_bookmarks!(Some("/"), None, 13); // 11 + 1 + 1

        info!("search bookmarks in root only");
        assert_searched_bookmarks!(Some("//"), None, 1);

        info!("search bookmarks in root only via cwd");
        assert_searched_bookmarks!(None, Some("//"), 1);
    }

    #[tokio::test]
    async fn search_bookmarks_in_folders_from_cwd() {
        let mut conn = connection::establish().await;

        let SetupFoldersAndBookmarksDefaultReturn {
            folder1_path,
            folder2_path,
            folder2_id,
            folder3_name,
            ..
        } = setup_folders_and_bookmarks_default(&mut conn).await;

        info!("search bookmarks with cwd");
        assert_searched_bookmarks!(None, Some(&folder1_path), 11); // 10 + 1
        assert_searched_bookmarks!(None, Some(&folder2_path), 1);

        let query = format!("./{}", folder3_name);
        info!(?query, "search bookmarks with cwd and relative path");
        assert_searched_bookmarks!(Some(&query), Some(&folder1_path), 1);

        let query = format!("./{} | ./", folder3_name);
        info!(?query, "search bookmarks with cwd and relative path");
        assert_searched_bookmarks!(Some(&query), Some(&folder1_path), 11); // 1 + 10

        let query = format!("./{} | .//", folder3_name);
        info!(?query, "search bookmarks with cwd and relative path");
        assert_searched_bookmarks!(Some(&query), Some(&folder1_path), 11); // 1 + 10

        let query = ".//";
        info!(?query, "search bookmarks with cwd and relative path");
        assert_searched_bookmarks!(Some(query), Some(&folder1_path), 10); // 10

        info!("search bookmarks with cwd but overwrite by absolute path");
        assert_searched_bookmarks!(Some(&folder2_path), Some(&folder1_path), 1);

        info!("search bookmark not in cwd");
        let bookmark = create_rand_bookmark(&mut conn).await;
        assert_searched_bookmarks!(Some(&bookmark.title), None, 1);
        assert_searched_bookmarks!(Some(&bookmark.title), Some(&folder2_path), 0);
        info!("search bookmark in cwd");
        move_bookmarks(&mut conn, folder2_id, &vec![bookmark.id])
            .await
            .unwrap();
        assert_searched_bookmarks!(Some(&bookmark.title), Some(&folder2_path), 1);
    }
}
