use bumpalo::boxed::Box as BBox;
use bumpalo::collections::String as BString;
use peggen::*;

pub use peggen::Parser;

#[derive(Debug, PartialEq, ParseImpl, Space, Num, EnumAstImpl)]
#[with(&'a bumpalo::Bump)]
pub enum Query<'a> {
    #[rule("{0:0} | {1:1}", group = 0)]
    Or(BBox<'a, Query<'a>>, BBox<'a, Query<'a>>),
    #[rule("{0:1} {1:2}", group = 1)]
    And(BBox<'a, Query<'a>>, BBox<'a, Query<'a>>),
    #[rule(r"( {0} )", group = 2)]
    Parenthesized(BBox<'a, Query<'a>>),
    #[rule(r#"#{0:`\w*`}"#, group = 2)]
    Tag(BString<'a>),
    #[rule(r#"{0:`\w+`}"#, group = 2)]
    Keyword(BString<'a>),
    #[rule(r#"{0:`(\.)?(/\w+)*/{0,2}`}"#, group = 3)]
    Path(BString<'a>),
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
#[ctor::ctor]
fn init() {
    use std::io;
    use tracing_subscriber::{prelude::*, EnvFilter};

    let console_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stdout)
        .boxed();

    tracing_subscriber::registry()
        .with(vec![console_log])
        .with(EnvFilter::from_default_env())
        .init();
}

#[cfg(test)]
mod test {
    use super::*;
    use Query::*;

    use tracing::info;

    #[test]
    fn test_primitive_path() {
        let bump = bumpalo::Bump::new();
        for src in [
            "/",          // root
            "./",         // relative root
            "/bar",       // decendent of /bar
            "/bar/",      // decendent of /bar
            "/bar/boo",   // decendent of /bar/boo
            "/bar/boo/",  // decendent of /bar/boo
            "./bar",      // decendent of ./bar
            "./bar/",     // decendent of ./bar
            "./bar/boo",  // decendent of ./bar/boo
            "./bar/boo/", // decendent of ./bar/boo
            "//",         // childern of root
            ".//",        // children of relative root
            "/boo//",     // children of /boot
            "./boo//",    // children of ./boo
        ] {
            let rv = Parser::<Query>::parse_with(src, &bump);
            info!(?rv, src, "parse result");
            assert!(rv.is_ok());
            assert_eq!(rv.unwrap(), Path(BString::from_str_in(src, &bump)));
        }
    }

    #[test]
    fn test_primitive_tag() {
        let bump = bumpalo::Bump::new();
        for src in [
            "",        // empty tag
            "foo",     // tag foo
            "foo_bar", // tag foo_bar
        ] {
            let rv = Parser::<Query>::parse_with(&format!("#{}", src), &bump);
            info!(?rv, src, "parse result");
            assert!(rv.is_ok());
            assert_eq!(rv.unwrap(), Tag(BString::from_str_in(src, &bump)));
        }
    }

    #[test]
    fn test_primitive_keyword() {
        let bump = bumpalo::Bump::new();
        for src in [
            "foo",     // keyword foo
            "foo_bar", // keyword foo_bar
        ] {
            let rv = Parser::<Query>::parse_with(src, &bump);
            info!(?rv, src, "parse result");
            assert!(rv.is_ok());
            assert_eq!(rv.unwrap(), Keyword(BString::from_str_in(src, &bump)));
        }
    }

    #[test]
    fn test_query_and() {
        let src = r#"#title | trust rust"#;
        let bump = bumpalo::Bump::new();
        let rv = Parser::<Query>::parse_with(src, &bump);
        info!(?rv, src, "parse result");
        assert!(rv.is_ok());
        assert_eq!(
            rv.unwrap(),
            Or(
                BBox::new_in(Tag(BString::from_str_in("title", &bump)), &bump),
                BBox::new_in(
                    And(
                        BBox::new_in(Keyword(BString::from_str_in("trust", &bump)), &bump),
                        BBox::new_in(Keyword(BString::from_str_in("rust", &bump)), &bump)
                    ),
                    &bump
                )
            )
        );
    }

    #[test]
    fn test_parsing() {
        let bump = bumpalo::Bump::new();
        for src in [
            r#"title #rust"#,
            r#"/cs/pl/rust"#,
            r#"/cs/pl title"#,
            r#"/cs/pl title #rust"#,
            r#"/cs title #rust"#,
            r#"title | #rust"#,
            r#"title | #rust #langs"#,
            r#"title ( #rust  #langs )"#,
            r#"title ( #rust | #langs )"#,
            r#"/blog/"#,
        ] {
            let rv = Parser::<Query>::parse_with(src, &bump);
            info!(?rv, ?src, "parsed");
            assert!(rv.is_ok());
        }
    }
}
