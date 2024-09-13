use pratt_gen::*;
use serde::Serializer;
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Query<'a> {
    #[parse("{0:2} | {1:1}", precedence = 2)]
    Or(&'a Self, &'a Self),
    #[parse("{0:4} {1:3}", precedence = 4)]
    And(&'a Self, &'a Self),
    #[parse("({0})")]
    Parenthesized(&'a Self),
    #[parse("{0}")]
    Primitive(Primitive<'a>),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Primitive<'a> {
    #[parse("{0}")]
    Path(Path<'a>),
    #[parse("#{0}")]
    Tag(Tag<'a>),
    #[parse("{0}")]
    Keyword(Keyword<'a>),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum Path<'a> {
    #[parse("./{0}")]
    Relative(&'a RelativePath<'a>),
    #[parse("/{0}")]
    Absolute(&'a RelativePath<'a>),
    #[parse("/")]
    Root(),
    #[parse("./")]
    CWD(),
}

#[derive(Debug, Clone, Copy, ParserImpl, Space)]
pub enum RelativePath<'a> {
    #[parse("{0}/{1}")]
    Join(Keyword<'a>, &'a Self),
    #[parse("{0}/")]
    NameEndSlash(Keyword<'a>),
    #[parse("{0}")]
    Name(Keyword<'a>),
    #[parse("/")] // for tailing "/", "//" ... syntax
    ExtraSlash(),
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

impl Display for Keyword<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keyword::Quoted(s) => write!(f, "\"{}\"", s),
            Keyword::Unquoted(s) => write!(f, "{}", s.0),
        }
    }
}

impl Display for Tag<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tag::Keyword(k) => write!(f, "#{}", k),
            Tag::Null() => Ok(()),
        }
    }
}

#[derive(Debug)]
pub struct QueryResult<'a> {
    pub tags: Vec<&'a str>,
    pub keywords: Vec<&'a str>,
    pub paths: Vec<&'a str>,
}

fn _path_to_str_parts<'a>(p: Path<'a>, parts: &mut Vec<&'a str>) {
    let mut cur = match p {
        Path::Root() => {
            parts.push("");
            parts.push("");
            return;
        }
        Path::Absolute(p) => {
            parts.push("");
            *p
        }
        Path::Relative(p) => {
            parts.push(".");
            *p
        }
        Path::CWD() => {
            parts.push(".");
            parts.push("");
            return;
        }
    };
    while match cur {
        RelativePath::Join(item, that) => {
            parts.push(item.into());
            cur = *that;
            true
        }
        RelativePath::Name(item) => {
            parts.push(item.into());
            false
        }
        RelativePath::NameEndSlash(item) => {
            parts.push(item.into());
            parts.push("");
            false
        }
        RelativePath::ExtraSlash() => {
            parts.push("");
            parts.push("");
            false
        }
    } {}
}

impl Display for Path<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = vec![];
        _path_to_str_parts(*self, &mut parts);
        write!(f, "{}", parts.join("/"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing::{debug, info};

    fn parse_query<'a>(raw: &'a str, out_arena: &'a Arena, err_arena: &'a Arena) -> Query<'a> {
        let source = Source::new(raw);
        let rv = parse::<Query>(source, out_arena, err_arena);
        debug!(?rv, ?source, "parsed");
        assert!(rv.is_ok());
        rv.unwrap()
    }

    #[test]
    fn test_parsing() {
        let out_arena = Arena::new();
        let err_arena = Arena::new();

        for src in [
            r#"/cs/pl/rust title #rust"#,
            r#"/cs/pl title #rust"#,
            r#"/cs title #rust"#,
            r#"title | #rust"#,
            r#"title | #rust #langs"#,
            r#"title ( #rust  #langs )"#,
            r#"title ( #rust | #langs )"#,
            r#"/blog/"#,
        ] {
            let rv = parse_query(src, &out_arena, &err_arena);
            info!(?rv, ?src, "parsed");
        }
    }
}
