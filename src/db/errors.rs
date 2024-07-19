use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("duplicate key value violates unique constraint of {table:?}")]
    DuplicationError { table: String },
}
