use dialoguer::{Confirm, Input, Select};
use eyre::{bail, eyre, Context, Result};
use log::{debug, info};
use scan_dir::ScanDir;
use std::cmp::Ordering;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Instant;

use crate::fetch::{get, get_cover, search, search_cover};
use crate::library::{LibraryRelease, LibraryTrack};
use crate::models::{Artists, GroupTracks, Release, Track, UNKNOWN_ARTIST};
use crate::rank::match_tracks;
use crate::theme::DialoguerTheme;
use crate::track::file::TrackFile;
use crate::track::picture::{write_picture, Picture, PictureType};
use crate::util::{mkdirp, path_to_str};
use crate::SETTINGS;

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
    } else {
        maybe_artists
    };
    let title = if maybe_titles.is_empty() {
        Input::new()
            .with_prompt("Insert a title for this release")
            .interact_text()?
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
    let (choice_release, choice_tracks) = tracks
        .clone()
        .group_tracks()
        .wrap_err("Trying to convert local files to internal structures")?;
    // TODO: reimplement artst & title manual input if they cannot be extracted from the tags
    // let (artists, title) = get_artist_and_title(&theme, ralbum.artists(), ralbum.titles())?;

    info!(
        "Searching for {} - {}...",
        choice_release.artists.joined(),
        choice_release.title
    );
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
    let (url, provider) = search_cover(&final_release.0).await?;
    info!("Found cover art from {:?}, converting...", provider);
    let (image, mime) = get_cover(url).await?;
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
    let picture = Picture {
        mime_type: mime,
        picture_type: PictureType::CoverFront,
        description: "Front".to_string(),
        data: image.clone(),
    };
    write_picture(&picture, &final_release.0)?;
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
        src.set_pictures(vec![picture.clone()])?;
        src.write()
            .wrap_err(eyre!("Could not write tags to track: {:?}", dest_path))?;
        debug!("After tagging {:?}", src);
    }

    info!("Import done, took {:?}", start.elapsed());
    Ok(())
}
