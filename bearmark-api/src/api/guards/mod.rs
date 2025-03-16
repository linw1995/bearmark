use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::api::configs::Config;
use crate::api::errors::Error;

pub struct Auth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth {
    type Error = Error;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        // Check if the user is authenticated
        if let Some(config) = request.rocket().state::<Config>() {
            if let Some(key) = config.api_key.as_ref() {
                let token = request.headers().get_one("Authorization");
                if let Some(token) = token {
                    if token != key {
                        return Outcome::Error((
                            Status::Forbidden,
                            Error::InvalidAPIKey("Invalid API Key".to_string()),
                        ));
                    }
                } else {
                    return Outcome::Error((
                        Status::Unauthorized,
                        Error::MissingAPIKey("Missing API Key".to_string()),
                    ));
                }
            }
            return Outcome::Success(Auth);
        };
        return Outcome::Error((
            Status::InternalServerError,
            Error::InternalServer("Missing Config".to_string()),
        ));
    }
}

#[cfg(test)]
mod test {
    use crate::utils::rand::rand_str;

    use super::*;

    use rocket::fairing::AdHoc;
    use rocket::http::Header;
    use rocket::local::blocking;

    #[get("/")]
    fn required_auth(_required: Auth) -> &'static str {
        "Hello, World!"
    }

    #[test]
    fn test_without_config() {
        let app = rocket::build().mount("/", routes![required_auth]);
        let client = blocking::Client::tracked(app).expect("valid rocket instance");
        let response = client.get(uri!(required_auth)).dispatch();
        assert_eq!(response.status(), Status::InternalServerError);
    }

    fn test_client(config: Config) -> blocking::Client {
        use rocket::figment::{Figment, providers::Serialized};
        let figment = Figment::from(rocket::Config::default()).merge(Serialized::defaults(config));
        let app = rocket::custom(figment)
            .mount("/", routes![required_auth])
            .attach(AdHoc::config::<Config>());
        blocking::Client::tracked(app).expect("valid rocket instance")
    }

    #[test]
    fn test_disable_auth() {
        let client = test_client(Config {
            api_key: None,
            ..Default::default()
        });
        let response = client.get(uri!(required_auth)).dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_enable_auth() {
        let key = rand_str(32);
        let client = test_client(Config {
            api_key: Some(key.clone()),
            ..Default::default()
        });

        let response = client.get(uri!(required_auth)).dispatch();
        assert_eq!(response.status(), Status::Unauthorized);

        let response = client
            .get(uri!(required_auth))
            .header(Header::new("Authorization", key))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client
            .get(uri!(required_auth))
            .header(Header::new("Authorization", rand_str(32)))
            .dispatch();
        assert_eq!(response.status(), Status::Forbidden);
    }
}
