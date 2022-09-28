use dialoguer::Confirm;
use eyre::{bail, eyre, Context, Result};
use log::{debug, info};
use scan_dir::ScanDir;
use std::cmp::Ordering;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Instant;

use crate::fetch::cover::{get_cover, search_covers};
use crate::fetch::{get, search};
use crate::library::Store;
use crate::library::{LibraryRelease, LibraryTrack};
use crate::models::{Artists, GroupTracks, Release, Track};
use crate::rank::{match_tracks, rank_covers};
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
    // see here for the previous implementation:
    // https://github.com/lucat1/tagger/blob/33fa9789ae4e38296edcdfc08270adda6c248529/src/import.rs#L33
    // Decide on what the user interaction should look like before proceeding with the implementation

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
    let mut covers_by_provider = search_covers(&final_release.0).await?;
    let cover = rank_covers(&mut covers_by_provider, &final_release.0)?;
    info!("Found cover art from {}, converting...", cover.provider);
    let (image, mime) = get_cover(cover.url).await?;
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
        dest.format = Some(src.format);
        let dest_path = dest.path()?;
        dest.path = Some(dest_path.clone());
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
        dest.store().await?;
        debug!("After tagging {:?}", src);
    }

    info!("Import done, took {:?}", start.elapsed());
    Ok(())
}
