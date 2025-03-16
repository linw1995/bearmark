use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::{
    Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
};

#[derive(Debug)]
struct PathsWithPrefix {
    prefix: LitStr,
    paths: Punctuated<Ident, Token![,]>,
}

impl Parse for PathsWithPrefix {
    fn parse(input: ParseStream) -> Result<Self> {
        let prefix = input.parse()?;
        input.parse::<Token![,]>()?;
        let paths = Punctuated::parse_separated_nonempty(input)?;
        Ok(Self { prefix, paths })
    }
}

#[proc_macro]
pub fn utoipa_paths(input: TokenStream) -> TokenStream {
    let rv: PathsWithPrefix = syn::parse(input).unwrap();

    let prefix = rv.prefix;

    let mut tailing_paths = quote! {};
    for ident in rv.paths {
        let ident = format_ident!("__path_{}", ident);
        tailing_paths.extend(quote! {
            .path(
                format!("{}{}", #prefix, #ident::path()),
                PathItem::from_http_methods(#ident::methods(), #ident::operation()),
            )
        });
    }

    let output = quote! {
        {
            use utoipa::openapi::{Paths, path::PathItem};

            let mut builder = Paths::builder();
            builder = builder #tailing_paths;

            builder.build()
        }
    };

    TokenStream::from(output)
}

#[proc_macro]
pub fn utoipa_components(input: TokenStream) -> TokenStream {
    let parser = Punctuated::<Ident, Token![,]>::parse_separated_nonempty;
    let idents = parser.parse(input).unwrap();
    let mut chaining = quote! {};
    for ident in idents {
        chaining.extend(quote! {
            builder = ComponentsBuilder::schema_from::<#ident>(builder);
        });
    }
    let output = quote! {
        {
            use utoipa::openapi::ComponentsBuilder;

            let mut builder = ComponentsBuilder::new();

            #chaining

            builder.build()
        }
    };
    TokenStream::from(output)
}

#[cfg(test)]
mod test {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn test_parse() {
        let rv: PathsWithPrefix = parse_quote! {
            "/api/v2", create_bookmark, delete_bookmark
        };
        println!("{:?}", rv);

        assert_eq!(rv.prefix.value(), "/api/v2");
        assert_eq!(
            rv.paths
                .into_iter()
                .map(|ident| ident.to_string())
                .collect::<Vec<_>>(),
            vec!["create_bookmark", "delete_bookmark"]
        );
    }
}
