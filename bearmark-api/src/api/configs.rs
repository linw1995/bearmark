use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    pub ui_path: Option<String>,

    pub api_key: Option<String>,
}
