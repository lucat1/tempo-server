mod files;
mod internal;
mod search_result;
mod track;

use base::util::UtilError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("Erorr during IO operation: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error during utility operation: {0}")]
    Util(#[from] UtilError),
    #[error("Error while scanning directory")]
    ScanDir(Vec<scan_dir::Error>),
}

pub use files::all_tracks;
pub use internal::{IntoInternal, UNKNOWN_ARTIST, UNKNOWN_TITLE};
pub use search_result::{CombinedSearchResults, SearchResult};
pub use track::TrackFile;
