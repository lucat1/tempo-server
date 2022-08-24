use eyre::{bail, eyre, Result};
use inquire::Select;
use log::{debug, info, trace};
use scan_dir::ScanDir;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Instant;

use crate::album::{FileAlbum, ReleaseLike};
use crate::fetch::{default_fetchers, search};
use crate::track::TrackFile;
use crate::util::path_to_str;

fn all_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    ScanDir::files()
        .walk(path_to_str(path)?, |iter| {
            iter.map(|(ref entry, _)| entry.path()).collect()
        })
        .map_err(|errs| match errs.first().map(|e| eyre!(e.to_string())) {
            Some(e) => e,
            None => eyre!("No errors"),
        })
}

pub async fn import(path: &PathBuf) -> Result<()> {
    let start = Instant::now();
    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) = files
        .iter()
        .map(|f| TrackFile::open(f))
        .partition(Result::is_ok);
    let tracks: Vec<_> = tracks.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();

    debug!("Found {} tracks, {} errors", tracks.len(), errors.len());
    if tracks.is_empty() && !errors.is_empty() {
        bail!(
            "Encountered an empty album folder but some files could not be analyzed:\n{:?}",
            errors
        )
    }
    let ralbum = FileAlbum::from_tracks(tracks)?;
    let artists = ralbum.artists()?;
    let titles = ralbum.titles()?;
    info!(
        "Importing {} files recursively from {}",
        ralbum.tracks.len(),
        path_to_str(path)?
    );
    debug!("Possible artists for album {:?}: {:?}", path, artists,);
    debug!("Possible titles for album {:?}: {:?}", path, titles,);
    let artist = if artists.len() <= 1 {
        artists
            .first()
            .ok_or(eyre!("Expected at least one album artist, found none"))?
            .clone()
    } else {
        Select::new("Album artist:", artists).prompt()?
    };
    let title = if titles.len() <= 1 {
        titles
            .first()
            .ok_or(eyre!("Expected at least one album title, found none"))?
            .clone()
    } else {
        Select::new("Album title:", titles).prompt()?
    };

    let res = search(default_fetchers(), artist, title, ralbum.tracks.len()).await?;
    for s in res {
        println!("- {:?} : {:?}", s.title(), s.artists());
    }

    trace!("Import for {:?} took {:?}", path, start.elapsed());
    Ok(())
}
