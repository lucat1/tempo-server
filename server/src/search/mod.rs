pub mod db;
pub mod documents;
mod schema;

pub use db::{get_indexes, open_index_writers, open_indexes, SearchError, INDEXES, INDEX_WRITERS};
pub use documents::{artist_fields, release_fields, track_fields};
