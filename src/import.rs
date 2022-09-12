use eyre::{bail, eyre, Context, Report, Result};
use inquire::{MultiSelect, Select};
use log::{debug, info};
use scan_dir::ScanDir;
use std::cmp::Ordering;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::fetch::{get, search};
use crate::library::{LibraryRelease, LibraryTrack};
use crate::models::{Artist, Artists, GroupTracks, Release, Track, UNKNOWN_ARTIST};
use crate::rank::match_tracks;
use crate::track::FileAlbum;
use crate::track::TrackFile;
use crate::util::{mkdirp, path_to_str};

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
            // TODO: consider reading mbid from files tag?
            // maybe an optin. Would make tagging really stale :/
            mbid: None,
            title: album.title,
            discs: album
                .tracks
                .iter()
                .filter_map(|t| t.clone().try_into().ok())
                .filter_map(|t: Track| t.disc)
                .max(),
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
        })
    }
}

impl GroupTracks for ChoiceAlbum {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)> {
        let tracks: Vec<Track> = self
            .tracks
            .iter()
            .map(|t| t.clone().try_into())
            .collect::<Result<Vec<_>>>()?;
        let rel: Release = self.try_into()?;
        let release = Some(Arc::new(rel.clone()));

        Ok((
            rel,
            tracks
                .into_iter()
                .map(|t| Track {
                    mbid: t.mbid,
                    title: t.title,
                    artists: t.artists,
                    length: t.length,
                    disc: t.disc,
                    disc_mbid: t.disc_mbid,
                    number: t.number,
                    genres: t.genres,
                    release: release.clone(),
                })
                .collect::<Vec<_>>(),
        ))
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
    let ralbum = FileAlbum::from_tracks(tracks.clone())?;
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
            .map_or(UNKNOWN_ARTIST.to_string(), |s| s.clone())
    } else {
        Select::new("Album title:", titles).prompt()?
    };

    let choice_album = ChoiceAlbum {
        title,
        artists,
        tracks: ralbum.tracks,
    };
    let (choice_release, choice_tracks) = choice_album
        .group_tracks()
        .wrap_err("Trying to convert local files to internal structures")?;
    let releases = search(&choice_release, choice_tracks.len())
        .await
        .wrap_err(eyre!("Error while fetching for album releases"))?;
    info!("Found {} release candidates, ranking...", releases.len());

    let mut expanded_releases: Vec<(Release, Vec<Track>)> = vec![];
    for release in releases.into_iter() {
        expanded_releases.push(get(&release).await?);
    }
    let mut rated_expanded_releases = expanded_releases
        .into_iter()
        .map(|(r, tracks)| (match_tracks(&choice_tracks, &tracks), (r, tracks)))
        .collect::<Vec<_>>();
    rated_expanded_releases.sort_by(|a, b| a.0 .0.partial_cmp(&b.0 .0).unwrap_or(Ordering::Equal));
    let ((_diff, tracks_map), final_release) = rated_expanded_releases
        .first()
        .ok_or(eyre!("No release available for given tracks"))?;
    info!(
        "Tagging as {} - {} ({})",
        final_release.0.artists.joined(),
        final_release.0.title,
        final_release
            .0
            .mbid
            .clone()
            .unwrap_or("no mbid".to_string()),
    );

    let dest = final_release.0.path()?;
    let other_paths = final_release.0.other_paths()?;
    debug!("Creating paths {:?}, {:?}", dest, other_paths);
    mkdirp(&dest)?;
    for path in other_paths.iter() {
        mkdirp(
            &path
                .parent()
                .ok_or(eyre!("Could not get parent of a release folder"))?
                .to_path_buf(),
        )?;

        #[cfg(target_os = "windows")]
        std::os::windows::fs::symlink_dir(&dest, path)?;

        #[cfg(not(target_os = "windows"))]
        std::os::unix::fs::symlink(&dest, path)?;
    }
    let mut final_tracks = tracks_map
        .into_iter()
        .enumerate()
        .map(|(i, map)| {
            info!("tracks_len: {}, i: {}, map: {}", tracks.len(), i, *map);
            (tracks[i].clone(), final_release.1[*map].clone())
        })
        .collect::<Vec<_>>();
    for (src, dest) in final_tracks.iter_mut() {
        let dest_path = dest.path(src.ext())?;
        info!("move {:?} to {:?}", src, dest_path);
        src.duplicate_to(&dest_path).wrap_err(eyre!(
            "Could not copy track {:?} to its new location: {:?}",
            src.path,
            dest_path
        ))?;
        src.clear()?;
        src.apply(dest.clone())
            .wrap_err(eyre!("Could not apply new tags to track: {:?}", dest_path))?;
        src.write()?;
        info!("new tags {:?}", src);
    }

    info!("Import for {:?} took {:?}", dest, start.elapsed());
    Ok(())
}
