mod db;
pub mod documents;
mod schema;

pub use db::{get_indexes, open_index_writers, open_indexes, INDEXES, INDEX_WRITERS};
