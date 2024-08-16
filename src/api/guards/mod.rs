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
