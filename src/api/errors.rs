use crate::utils::{BearQLError, DatabaseError};

#[derive(Responder)]
pub enum Error {
    #[response(status = 404)]
    NotFound(String),
    #[response(status = 400)]
    BadRequest(String),
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
