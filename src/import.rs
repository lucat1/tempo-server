use std::path::PathBuf;
use std::fs::canonicalize;
use eyre::{eyre, Result, bail};
use scan_dir::ScanDir;

use crate::tag::Track;
use crate::util::path_to_str;

fn all_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    ScanDir::files().read(path_to_str(path)?, |iter| {
        iter.map(|(entry, _)| entry.path()).collect()
    }).map_err(|err| eyre!(err))
}

pub fn import(path: &PathBuf) -> Result<()> {
    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) = files.iter().map(|f| Track::open(f)).partition(Result::is_ok);
    let tracks: Vec<_> = tracks.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();

    if tracks.is_empty() && !errors.is_empty() {
        bail!("Encountered an empty album folder but some files could not be analyzed:\n{:?}", errors)
    }
    for track in tracks {
        println!("track: {:#?}", track)
    }
    Ok(())
}
