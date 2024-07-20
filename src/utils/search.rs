use itertools::Itertools;
use pratt_gen::*;
use serde::Serializer;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Query<'a> {
    #[parse("{0} {1}")]
    And(Primitive<'a>, &'a Query<'a>),
    #[parse("{0}")]
    Primitive(Primitive<'a>),
    #[parse("")]
    Null(),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Primitive<'a> {
    #[parse("/{0}")]
    Path(Path<'a>),
    #[parse("#{0}")]
    Tag(Tag<'a>),
    #[parse("{0}")]
    Keyword(Keyword<'a>),
}

#[derive(Clone, Copy, ParserImpl, Space)]
pub enum Path<'a> {
    #[parse("{0}/{1}")]
    Join(Keyword<'a>, &'a Path<'a>),
    #[parse("{0}")]
    Name(Keyword<'a>),
    #[parse("/")] // for "//" syntax
    JoinJoin(),
    #[parse("")]
    Empty(),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Tag<'a> {
    #[parse("{0}")]
    Keyword(Keyword<'a>),
    #[parse("")]
    Null(),
}

#[derive(Clone, Copy, ParserImpl, Space)]
pub enum Keyword<'a> {
    #[parse("{0}")]
    Quoted(&'a str),
    #[parse("{0}")]
    Unquoted(&'a Ident<'a>),
}

impl<'a> Debug for Keyword<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.serialize_str(match self {
            Self::Quoted(s) => s,
            Self::Unquoted(s) => s.0,
        })
    }
}

impl<'a> Debug for Path<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.serialize_str(format!("/{}", self.into_iter().join("/")).as_str())
    }
}

impl<'a> From<Keyword<'a>> for &'a str {
    fn from(k: Keyword<'a>) -> &'a str {
        match k {
            Keyword::Quoted(s) => s,
            Keyword::Unquoted(s) => s.0,
        }
    }
}

impl<'a> From<Tag<'a>> for &'a str {
    fn from(t: Tag<'a>) -> &'a str {
        match t {
            Tag::Keyword(k) => k.into(),
            Tag::Null() => "",
        }
    }
}

impl<'a> Iterator for Query<'a> {
    type Item = Primitive<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::And(item, that) => {
                *self = *that;
                Some(item)
            }
            Self::Primitive(item) => {
                *self = Self::Null();
                Some(item)
            }
            Self::Null() => None,
        }
    }
}

impl<'a> Iterator for Path<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::Join(item, that) => {
                *self = *that;
                Some(item.into())
            }
            Self::JoinJoin() => {
                *self = Self::Empty();
                Some("/")
            }
            Self::Name(item) => {
                *self = Self::Empty();
                Some(item.into())
            }
            Self::Empty() => None,
        }
    }
}

#[derive(Debug)]
pub struct QueryResult<'a> {
    pub tags: Vec<&'a str>,
    pub keywords: Vec<&'a str>,
    pub paths: Vec<&'a str>,
}

fn path_to_str<'a>(p: Path<'a>, arena: &'a Arena) -> &'a str {
    unsafe { arena.alloc_str(&p.into_iter().join("/")) }
}

pub fn parse_query<'a>(
    raw: &'a str,
    out_arena: &'a Arena,
    err_arena: &'a Arena,
) -> Result<QueryResult<'a>, Error<'a>> {
    let source = Source::new(raw);
    let rv = parse::<Query>(source, out_arena, err_arena)?;
    let rv = rv.collect::<Vec<Primitive>>();

    let mut tags = vec![];
    let mut keywords = vec![];
    let mut paths = vec![];

    for primitive in rv {
        match primitive {
            Primitive::Tag(tag) => tags.push(tag.into()),
            Primitive::Keyword(keyword) => keywords.push(keyword.into()),
            Primitive::Path(path) => paths.push(path_to_str(path, out_arena)),
        }
    }

    Ok(QueryResult {
        tags,
        keywords,
        paths,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing::{debug, info};

    fn to_str_vec<'a>(primitives: Vec<Primitive<'a>>, arena: &'a Arena) -> Vec<&'a str> {
        primitives
            .into_iter()
            .map(|x| match x {
                Primitive::Path(path) => path_to_str(path, &arena),
                Primitive::Tag(tag) => tag.into(),
                Primitive::Keyword(keyword) => keyword.into(),
            })
            .collect()
    }

    fn parse_query<'a>(
        raw: &'a str,
        out_arena: &'a Arena,
        err_arena: &'a Arena,
    ) -> Vec<Primitive<'a>> {
        let source = Source::new(raw);
        let rv = parse::<Query>(source, &out_arena, &err_arena);
        debug!(?rv, "parsed");
        assert!(rv.is_ok());
        let rv = rv.unwrap().collect::<Vec<Primitive>>();
        debug!(?rv, "collect into vec");
        rv
    }

    #[test]
    fn query_with_tags() {
        let out_arena = Arena::new();
        let err_arena = Arena::new();

        for (raw, expect_tags, expect_keywords) in vec![
            ("", vec![], vec![]),
            (r#"rust"#, vec![], vec!["rust"]),
            (r#""rust pl""#, vec![], vec!["rust pl"]),
            (r#"#rust"#, vec!["rust"], vec![]),
            (r#"#"#, vec![""], vec![]),
            (r#"# title"#, vec![""], vec!["title"]),
            (r#"#"special tag""#, vec!["special tag"], vec![]),
            (
                r#"#rust #pl title "my title" #"I'm" "#,
                vec!["rust", "pl", "I'm"],
                vec!["title", "my title"],
            ),
        ] {
            info!(?raw, ?expect_tags, ?expect_keywords, "testing query");
            let rv = parse_query(raw, &out_arena, &err_arena);

            let (tags, keywords) = rv.into_iter().partition(|x| matches!(x, Primitive::Tag(_)));
            let tags = to_str_vec(tags, &out_arena);
            let keywords = to_str_vec(keywords, &out_arena);
            debug!(?tags, ?keywords, "the query conditions for where clause");

            assert_eq!(tags, expect_tags);
            assert_eq!(keywords, expect_keywords);
        }
    }

    #[test]
    fn query_in_unusually_path() {
        let out_arena = Arena::new();
        let err_arena = Arena::new();

        let rv = parse_query(r#"// title #rust "#, &out_arena, &err_arena);
        assert_eq!(rv.len(), 3);
        assert!(matches!(rv[0], Primitive::Path(Path::JoinJoin())));

        let rv = parse_query(r#"/ title #rust "#, &out_arena, &err_arena);
        assert_eq!(rv.len(), 3);
        assert!(matches!(rv[0], Primitive::Path(Path::Empty())));
    }

    #[test]
    fn query_in_path() {
        let out_arena = Arena::new();
        let err_arena = Arena::new();

        for src in vec![
            r#"/cs/pl/rust title #rust"#,
            r#"/cs/pl title #rust"#,
            r#"/cs title #rust"#,
        ] {
            let _rv = parse_query(&src, &out_arena, &err_arena);
            // TODO: assert_eq!(rv, expect);
        }
    }
}
