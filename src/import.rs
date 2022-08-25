use eyre::{bail, eyre, Context, Result};
use inquire::{MultiSelect, Select};
use log::{debug, info, trace};
use scan_dir::ScanDir;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Instant;

use crate::album::FileAlbum;
use crate::fetch::{default_fetchers, get, search, ArtistLike, Fetch, ReleaseLike};
use crate::rank::rate;
use crate::track::{TrackFile, TrackLike};
use crate::util::path_to_str;

const TRY_RELEASE_COUNT: usize = 5;

#[derive(Clone, Debug)]
struct ChoiceAlbum {
    artists: Vec<String>,
    title: String,
    tracks: Vec<TrackFile>,
}

impl ReleaseLike for ChoiceAlbum {
    fn fetcher(&self) -> Option<Box<dyn Fetch>> {
        None
    }
    fn id(&self) -> Option<String> {
        None
    }

    fn artists(&self) -> Vec<Box<dyn ArtistLike>> {
        self.artists
            .iter()
            .map(|a| Box::new(a.clone()) as Box<dyn ArtistLike>)
            .collect()
    }

    fn title(&self) -> String {
        self.title.clone()
    }
    fn tracks(&self) -> Option<Vec<Box<dyn TrackLike>>> {
        Some(
            self.tracks
                .iter()
                .map(|t| Box::new(t.clone()) as Box<dyn TrackLike>)
                .collect(),
        )
    }
}

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
    if !errors.is_empty() {
        errors
            .iter()
            .for_each(|e| debug!("Error while importing file:{}", e));
    }

    if tracks.is_empty() && !errors.is_empty() {
        bail!(
            "Encountered an empty album folder but some files could not be analyzed:\n{:?}",
            errors
        )
    }
    let ralbum = FileAlbum::from_tracks(tracks)?;
    let rartists = ralbum.artists()?;
    let titles = ralbum.titles()?;
    info!(
        "Importing {} files from {}",
        ralbum.tracks.len(),
        path_to_str(path)?
    );
    debug!("Possible artists for album {:?}: {:?}", path, rartists);
    debug!("Possible titles for album {:?}: {:?}", path, titles);
    if rartists.len() < 1 {
        bail!("Expected at least one album artist, found none")
    }
    let artists = if rartists.len() == 1 {
        rartists
    } else {
        MultiSelect::new("Album artist(s):", rartists).prompt()?
    };
    let title = if titles.len() <= 1 {
        titles
            .first()
            .ok_or(eyre!("Expected at least one album title, found none"))?
            .clone()
    } else {
        Select::new("Album title:", titles).prompt()?
    };

    let choice_album = Box::new(ChoiceAlbum {
        title,
        artists,
        tracks: ralbum.tracks,
    });
    let releases = search(default_fetchers(), choice_album.clone())
        .await
        .wrap_err(eyre!("Error while fetching for album releases"))?;
    info!("Found {} release candicates, ranking...", releases.len());

    let mut rated_releases = releases
        .iter()
        .map(|r| (rate(choice_album.clone(), r.clone()), r.clone()))
        .collect::<Vec<_>>();
    rated_releases.sort_by(|a, b| b.0 .0.partial_cmp(&a.0 .0).unwrap());
    rated_releases = rated_releases.as_slice()[0..TRY_RELEASE_COUNT].to_vec();
    for s in rated_releases.clone() {
        info!(
            "- {}: {:?} - {:?} ({:?})",
            s.0 .0,
            s.1.title(),
            s.1.artists().iter().map(|a| a.name()).collect::<Vec<_>>(),
            s.1.id()
        );
    }
    let mut expanded_releases: Vec<Box<dyn ReleaseLike>> = vec![];
    for release in rated_releases {
        expanded_releases.push(get(release.1.clone()).await?);
    }
    let mut rated_expanded_releases = expanded_releases
        .iter()
        .map(|r| (rate(choice_album.clone(), r.clone()), r.clone()))
        .collect::<Vec<_>>();
    for s in rated_expanded_releases.clone() {
        info!(
            "- {}: {:?} - {:?} ({:?}) len {}",
            s.0 .0,
            s.1.title(),
            s.1.artists().iter().map(|a| a.name()).collect::<Vec<_>>(),
            s.1.id(),
            match s.1.tracks() {
                None => 0,
                Some(t) => t.len(),
            }
        );
    }

    trace!("Import for {:?} took {:?}", path, start.elapsed());
    Ok(())
}
