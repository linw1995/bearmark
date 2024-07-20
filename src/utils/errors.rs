use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("duplicate key value violates unique constraint of {table:?}")]
    DuplicationError { table: String },
}

#[derive(Error, Debug)]
pub enum BearQLError {
    #[error("Syntax error: {msg}")]
    SyntaxError {
        msg: String,
        ql: String,
        err_msg: String,
    },
    #[error("Empty tag name error")]
    EmptyTag,
    #[error("Empty keyword error")]
    EmptyKeyword,
}
