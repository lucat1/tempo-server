use dialoguer::{Confirm, Input, MultiSelect, Select};
use eyre::{bail, eyre, Context, Report, Result};
use log::{debug, info, warn};
use scan_dir::ScanDir;
use std::cmp::Ordering;
use std::fs::canonicalize;
use std::iter::repeat;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::fetch::{get, search};
use crate::library::{LibraryRelease, LibraryTrack};
use crate::models::{Artist, Artists, GroupTracks, Release, Track, UNKNOWN_ARTIST};
use crate::rank::match_tracks;
use crate::theme::DialoguerTheme;
use crate::track::FileAlbum;
use crate::track::TrackFile;
use crate::util::{mkdirp, path_to_str};
use crate::SETTINGS;

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
            release_group_mbid: None,
            title: album.title,
            tracks: Some(album.tracks.len() as u64),
            discs: album
                .tracks
                .iter()
                .filter_map(|t| t.clone().try_into().ok())
                .filter_map(|t: Track| t.disc)
                .max(),
            media: None,
            // TODO: as part of removing this structure to somewhere else,
            // make sense of where it is more appropriate to fetch this kind of
            // data (answer: here in the try_from) from a Vec<TrackFile> and
            // move all the data gathering here (fetching title and artists)
            asin: None,
            country: None,
            label: None,
            catalog_no: None,
            status: None,
            release_type: None,
            date: None,
            original_date: None,
            script: None,
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
                    // TODO
                    instruments: vec![],
                })
                .collect::<Vec<_>>(),
        })
    }
}

impl GroupTracks for ChoiceAlbum {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)> {
        let mut tracks: Vec<Track> = self
            .tracks
            .iter()
            .map(|t| t.clone().try_into())
            .collect::<Result<Vec<_>>>()?;
        let rel: Release = self.try_into()?;
        let release = Some(Arc::new(rel.clone()));

        for track in tracks.iter_mut() {
            track.release = release.clone();
        }
        Ok((rel, tracks))
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

fn get_artist_and_title(
    theme: &DialoguerTheme,
    maybe_artists: Vec<String>,
    maybe_titles: Vec<String>,
) -> Result<(Vec<String>, String)> {
    debug!("Possible artists for album: {:?}", maybe_artists);
    debug!("Possible titles for album: {:?}", maybe_titles);
    let artists = if maybe_artists.is_empty() {
        vec![UNKNOWN_ARTIST.to_string()]
    } else if maybe_artists.len() == 1 {
        maybe_artists
    } else {
        match MultiSelect::with_theme(theme)
            .with_prompt("Which album artist should be used?")
            .items(&maybe_artists)
            .defaults(&repeat(true).take(maybe_artists.len()).collect::<Vec<_>>())
            .interact_opt()?
            .map_or(None, |v| if v.len() > 0 { Some(v) } else { None })
        {
            Some(v) => v.into_iter().map(|i| maybe_artists[i].clone()).collect(),
            None => {
                warn!("No artist chosen. Using: \"{}\"", UNKNOWN_ARTIST);
                vec![UNKNOWN_ARTIST.to_string()]
            }
        }
    };
    let title = if maybe_titles.is_empty() {
        Input::new().interact_text()?
    } else if maybe_titles.len() == 1 {
        maybe_titles.first().unwrap().to_string()
    } else {
        let index = match Select::with_theme(theme)
            .with_prompt("What's the title of the release?")
            .items(&maybe_titles)
            .default(0)
            .interact_opt()?
        {
            Some(v) => v,
            None => bail!("No album title selected"),
        };
        maybe_titles[index].to_string()
    };
    Ok((artists, title))
}

pub async fn import(path: &PathBuf) -> Result<()> {
    let start = Instant::now();
    let settings = SETTINGS.get().ok_or(eyre!("Could not read settings"))?;
    let theme = DialoguerTheme::default();

    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) = files
        .iter()
        .map(|f| TrackFile::open(f))
        .partition(Result::is_ok);
    let tracks: Vec<_> = tracks.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();
    debug!("Found {} tracks, {} errors", tracks.len(), errors.len());
    info!("Importing {} audio files from {:?}", tracks.len(), path);

    if !errors.is_empty() {
        errors
            .iter()
            .for_each(|e| debug!("Error while importing file:{}", e));
    }
    if tracks.is_empty() {
        bail!("No tracks to import were found");
    }
    let ralbum = FileAlbum::from_tracks(tracks.clone())?;
    let (artists, title) = get_artist_and_title(&theme, ralbum.artists(), ralbum.titles())?;

    let choice_album = ChoiceAlbum {
        title,
        artists,
        tracks: ralbum.tracks,
    };
    info!(
        "Searching for {} - {}...",
        choice_album.artists.join(", "),
        choice_album.title
    );
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
    if !Confirm::with_theme(&theme)
        .with_prompt("Proceed?")
        .interact()?
    {
        bail!("Aborted")
    }

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
        .map(|(i, map)| (tracks[i].clone(), final_release.1[*map].clone()))
        .collect::<Vec<_>>();
    for (src, dest) in final_tracks.iter_mut() {
        debug!("Beofre tagging {:?}", src);
        let dest_path = dest.path(src.ext())?;
        src.duplicate_to(&dest_path).wrap_err(eyre!(
            "Could not copy track {:?} to its new location: {:?}",
            src.path,
            dest_path
        ))?;
        if settings.tagging.clear {
            src.clear()
                .wrap_err(eyre!("Could not celar tracks from file: {:?}", dest_path))?;
        }
        src.apply(dest.clone())
            .wrap_err(eyre!("Could not apply new tags to track: {:?}", dest_path))?;
        src.write()
            .wrap_err(eyre!("Could not write tags to track: {:?}", dest_path))?;
        debug!("After tagging {:?}", src);
    }

    info!("Import done, took {:?}", start.elapsed());
    Ok(())
}
