pub mod bookmark;
pub mod errors;
pub mod folder;
pub mod tag;

#[cfg(feature = "diesel")]
pub mod schema;

#[cfg(feature = "diesel")]
pub use schema::*;

// Re-export for convenience
pub use bookmark::{Bookmark, CreateBookmark, ModifyBookmark};
pub use errors::*;
pub use folder::CreateFolder;
