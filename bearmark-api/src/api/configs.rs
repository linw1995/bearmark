use rocket::{
    figment::Figment,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    pub ui_path: Option<String>,
    pub api_key: Option<String>,
}

pub fn config_provider() -> Figment {
    use rocket::figment::providers::{Env, Serialized};

    rocket::figment::Figment::from(rocket::Config::default())
        .merge(Serialized::defaults(Config::default()))
        .merge(("databases.main", rocket_db_pools::Config::default()))
        .merge(Env::prefixed("BM_").global())
}

pub fn get_database_url() -> String {
    config_provider()
        .extract_inner("databases.main.url")
        .unwrap()
}
