#[macro_use]
extern crate rocket;

mod api;
mod db;
mod utils;

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
#[ctor::ctor]
fn init() {
    crate::utils::logging::setup_console_log();
}

pub(crate) mod misc {
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;
    use utoipa_swagger_ui::Url;

    #[derive(OpenApi)]
    #[openapi(info(
        title = "Bearmark API",
        description = r"## Main API documentation

- [Bookmarks API](/swagger-ui/?urls.primaryName=bookmarks)
- [Folders API](/swagger-ui/?urls.primaryName=folders)
    ",
        version = "1.0"
    ))]
    pub struct ApiDoc;

    pub fn docs() -> Vec<rocket::Route> {
        use crate::api::{bookmark, folder};
        SwaggerUi::new("/swagger-ui/<_..>")
            .urls(vec![
                (
                    Url::with_primary("main", "/api-docs/openapi.json", true),
                    ApiDoc::openapi(),
                ),
                (
                    Url::new("bookmarks", "/api-docs/openapi-bookmarks.json"),
                    bookmark::misc::ApiDoc::openapi(),
                ),
                (
                    Url::new("folders", "/api-docs/openapi-folders.json"),
                    folder::misc::ApiDoc::openapi(),
                ),
            ])
            .into()
    }
}

#[cfg(not(tarpaulin_include))]
pub async fn rocket() -> rocket::Rocket<rocket::Build> {
    use rocket::fairing::AdHoc;
    use rocket::fs::FileServer;
    use rocket_db_pools::Database;

    use crate::api::configs::{self, Config};
    use crate::api::fairings::db::Db;
    use crate::api::{bookmark, folder, tag};
    use crate::misc;

    crate::utils::logging::setup_console_log();
    crate::db::connection::run_migrations().await;

    let cfg_provider = configs::config_provider();
    let ui_path = cfg_provider
        .extract_inner::<Option<String>>("ui_path")
        .unwrap();

    let mut builder = rocket::custom(cfg_provider);
    if let Some(ui_path) = ui_path {
        // Serve the UI files if the path is provided
        builder = builder.mount("/", FileServer::from(ui_path));
    }
    builder
        .attach(Db::init())
        .mount("/api/bookmarks", bookmark::routes())
        .mount("/api/tags", tag::routes())
        .mount("/api/folders", folder::routes())
        .mount("/", misc::docs())
        .attach(AdHoc::config::<Config>())
}
