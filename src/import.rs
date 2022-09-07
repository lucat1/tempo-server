use eyre::{bail, eyre, Context, Report, Result};
use inquire::{MultiSelect, Select};
use log::{debug, info};
use scan_dir::ScanDir;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Instant;

use crate::album::FileAlbum;
use crate::fetch::{default_fetchers, get, search};
use crate::models::{Artist, Release};
use crate::rank::match_tracks;
use crate::track::TrackFile;
use crate::util::path_to_str;

const TRY_RELEASE_COUNT: usize = 5;

#[derive(Clone, Debug)]
// TODO: make the structure more complex to extract more data from the tags
// i.e. mbids, join phrase, sort name for artists
//      mbid for the album *maybe*
struct ChoiceAlbum {
    artists: Vec<String>,
    title: String,
    tracks: Vec<TrackFile>,
}

impl TryFrom<ChoiceAlbum> for Release {
    type Error = Report;
    fn try_from(album: ChoiceAlbum) -> Result<Self> {
        Ok(Release {
            fetcher: None,
            // TODO: consider reading mbid from files tag?
            // maybe an optin. Would make tagging really stale :/
            mbid: None,
            title: album.title,
            artists: album
                .artists
                .iter()
                .map(|a| Artist {
                    mbid: None,
                    // TODO
                    name: a.to_string(),
                    // TODO
                    join_phrase: None,
                    sort_name: None,
                })
                .collect::<Vec<_>>(),
            tracks: album
                .tracks
                .iter()
                .map(|t| t.clone().try_into())
                .collect::<Result<Vec<_>>>()?,
        })
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

    let choice_album = ChoiceAlbum {
        title,
        artists,
        tracks: ralbum.tracks,
    };
    let choice_release: Release = choice_album
        .try_into()
        .wrap_err("Trying to convert local files to internal structures")?;
    let releases = search(default_fetchers(), choice_release.clone())
        .await
        .wrap_err(eyre!("Error while fetching for album releases"))?;
    info!("Found {} release candidates, ranking...", releases.len());

    // let mut rated_releases = releases
    //     .iter()
    //     .map(|r| (rate(choice_album.clone(), r.clone()), r.clone()))
    //     .collect::<Vec<_>>();
    // rated_releases.sort_by(|a, b| b.0 .0.partial_cmp(&a.0 .0).unwrap());
    let rated_releases = releases.as_slice()[0..TRY_RELEASE_COUNT].to_vec();
    for s in rated_releases.clone() {
        info!(
            "- {:?} - {:?} ({:?})",
            s.title,
            s.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
            s.mbid
        );
    }
    let mut expanded_releases: Vec<Release> = vec![];
    for release in rated_releases {
        expanded_releases.push(get(release.clone()).await?);
    }
    let mut rated_expanded_releases = expanded_releases
        .iter()
        .map(|r| (match_tracks(&choice_release.tracks, &r.tracks), r.clone()))
        .collect::<Vec<_>>();
    rated_expanded_releases.sort_by(|a, b| a.0 .0.partial_cmp(&b.0 .0).unwrap());
    for s in rated_expanded_releases.clone() {
        info!(
            "- {}: {:?} - {:?} ({:?}) len {}",
            s.0 .0,
            s.1.title,
            s.1.artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>(),
            s.1.mbid,
            s.1.tracks.len()
        );
    }

    info!("Import for {:?} took {:?}", path, start.elapsed());
    Ok(())
}
