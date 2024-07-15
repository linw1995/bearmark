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
    #[parse("# {0}")]
    Tag(Keyword<'a>),
    #[parse("{0}")]
    Keyword(Keyword<'a>),
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

impl<'a> From<Primitive<'a>> for &'a str {
    fn from(p: Primitive<'a>) -> &'a str {
        match p {
            Primitive::Tag(k) => k.into(),
            Primitive::Keyword(k) => k.into(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn query() {
        let source = Source::new(r#"#rust #pl title "my title" #"I'm" "#);
        let out_arena = Arena::new();
        let err_arena = Arena::new();
        let rv = parse::<Query>(source, &out_arena, &err_arena);
        println!("{:?}", rv);
        assert!(rv.is_ok());

        let rv = rv.unwrap().collect::<Vec<Primitive>>();
        println!("{:?}", rv);

        let (tags, keywords): (Vec<_>, Vec<_>) =
            rv.into_iter().partition(|x| matches!(x, Primitive::Tag(_)));

        assert_eq!(tags.len(), 3);
        assert_eq!(keywords.len(), 2);
        println!(
            "tags: {:?}",
            tags.into_iter().map(|x| x.into()).collect::<Vec<&str>>()
        );
        println!(
            "keywords: {:?}",
            keywords
                .into_iter()
                .map(|x| x.into())
                .collect::<Vec<&str>>()
        );
    }
}
