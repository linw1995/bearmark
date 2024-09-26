use crate::utils::{BearQLError, CommonError, DatabaseError};

#[derive(Responder, Debug)]
pub enum Error {
    #[response(status = 404)]
    NotFound(String),
    #[response(status = 400)]
    BadRequest(String),
    #[response(status = 500)]
    InternalServer(String),

    #[response(status = 401)]
    MissingAPIKey(String),
    #[response(status = 403)]
    InvalidAPIKey(String),
}

impl From<DatabaseError> for Error {
    fn from(e: DatabaseError) -> Self {
        match e {
            DatabaseError::DuplicationError { table: _ } => Error::BadRequest(e.to_string()),
            DatabaseError::ViolationError() => Error::BadRequest(e.to_string()),
        }
    }
}

impl From<BearQLError> for Error {
    fn from(e: BearQLError) -> Self {
        match e {
            BearQLError::SyntaxError {
                msg,
                ql: _,
                err_msg: _,
            } => Error::BadRequest(format!("Syntax Error: {}", msg)),
            BearQLError::EmptyKeyword => Error::BadRequest("Empty keyword error".to_string()),
            BearQLError::EmptyTag => Error::BadRequest("Empty tag name error".to_string()),
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Self {
        Error::InternalServer(e.to_string())
    }
}

impl From<CommonError> for Error {
    fn from(e: CommonError) -> Self {
        match e {
            CommonError::InvalidCWD => Error::BadRequest("Invalid CWD".to_string()),
            CommonError::BearQL(e) => Error::from(e),
        }
    }
}
