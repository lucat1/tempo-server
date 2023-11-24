use scan_dir::ScanDir;
use std::{
    fs::canonicalize,
    path::{Path, PathBuf},
};

use super::{ImportError, TrackFile};
use base::{setting::Library, util::path_to_str};

fn all_files(path: &Path) -> Result<Vec<PathBuf>, ImportError> {
    ScanDir::files()
        .walk(path_to_str(path)?, |iter| {
            iter.map(|(ref entry, _)| entry.path()).collect()
        })
        .map_err(ImportError::ScanDir)
}

pub async fn all_tracks(library: &Library, path: &Path) -> Result<Vec<TrackFile>, ImportError> {
    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) = files
        .iter()
        .map(|f| TrackFile::open(library, f))
        .partition(Result::is_ok);
    let tracks: Vec<_> = tracks.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();
    tracing::info! {
        tracks=%tracks.len(),
        errors=%errors.len(),
        "Found tracks, with files ignored due to errors"
    };
    if !errors.is_empty() {
        errors
            .iter()
            .for_each(|error| tracing::trace! {%error, "Error while importing file"});
    }
    Ok(tracks)
}
