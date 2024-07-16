pub mod logging;
pub mod rand;
pub mod search;

pub fn percent_encoding(query: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

    utf8_percent_encode(query, NON_ALPHANUMERIC).to_string()
}
