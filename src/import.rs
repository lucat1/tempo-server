use eyre::{bail, eyre, Result};
use scan_dir::ScanDir;
use std::fs::canonicalize;
use std::path::PathBuf;
use tokio::runtime::Handle;

use crate::album::{AlbumLike, RoughAlbum};
use crate::fetch::{default_fetchers, search};
use crate::track::TrackFile;
use crate::util::path_to_str;

fn all_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    ScanDir::files()
        .read(path_to_str(path)?, |iter| {
            iter.map(|(entry, _)| entry.path()).collect()
        })
        .map_err(|err| eyre!(err))
}

pub async fn import(path: &PathBuf) -> Result<()> {
    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) = files
        .iter()
        .map(|f| TrackFile::open(f))
        .partition(Result::is_ok);
    let tracks: Vec<_> = tracks.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();

    if tracks.is_empty() && !errors.is_empty() {
        bail!(
            "Encountered an empty album folder but some files could not be analyzed:\n{:?}",
            errors
        )
    }
    let ralbum = RoughAlbum::from_tracks(tracks)?;
    println!("possible titles {:?}", ralbum.title());
    println!("possible artists {:?}", ralbum.artist());

    let res = search(default_fetchers(), Box::new(ralbum)).await?;
    println!("search results: {:?}", res);

    Ok(())
}
