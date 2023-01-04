use dialoguer::{Confirm, Input, Select};
use eyre::{bail, eyre, Context, Result};
use log::{debug, info, warn};
use scan_dir::ScanDir;
use std::cmp::Ordering;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::Instant;

use crate::fetch::cover::{get_cover, search_covers};
use crate::fetch::structures::Cover;
use crate::fetch::{get, search};
use crate::library::LibraryTrack;
use crate::library::Store;
use crate::models::{Artists, GroupTracks, Release, Track};
use crate::rank::CoverRating;
use crate::rank::{match_tracks, rank_covers};
use crate::theme::DialoguerTheme;
use crate::track::file::TrackFile;
use crate::track::picture::{write_picture, Picture, PictureType};
use crate::util::{mkdirp, path_to_str};
use crate::{DB, SETTINGS};

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

async fn ask(
    theme: &DialoguerTheme,
    original_tracks: &Vec<Track>,
    candidate: (Release, Vec<Track>, Vec<usize>),
) -> Result<(bool, Release, Vec<Track>, Vec<usize>)> {
    info!(
        "Tagging as {} - {} ({})",
        candidate.0.artists.joined(),
        candidate.0.title,
        candidate
            .0
            .mbid
            .clone()
            .unwrap_or_else(|| "no mbid".to_string()),
    );
    let ch = Input::<char>::with_theme(theme)
        .with_prompt("Proceed? [y]es, [n]o, [i]d")
        .interact()
        .map_err(|_| eyre!("Aborted"))?;
    match ch {
        'y' => Ok((true, candidate.0, candidate.1, candidate.2)),
        'n' => Err(eyre!("Aborted")),
        'i' => {
            let id: String = Input::with_theme(theme)
                .with_prompt("Enter the MusicBrainz Release ID")
                .interact()
                .map_err(|_| eyre!("Aborted"))?;
            let (release, tracks) = get(id.as_str()).await?;
            let (_, tracks_map) = match_tracks(original_tracks, &tracks);
            Ok((false, release, tracks, tracks_map))
        }
        v => {
            warn!("Invalid choice: {}", v);
            Ok((false, candidate.0, candidate.1, candidate.2))
        }
    }
}

fn ask_cover(theme: &DialoguerTheme, covers: Vec<CoverRating>) -> Option<Cover> {
    let CoverRating(match_rank, mut cover) = covers.first()?.clone();
    let mut index: usize = 0;
    info!(
        "Using cover art for release {} - {} from {} ({}x{}, diff: {})",
        cover.artist, cover.title, cover.provider, cover.width, cover.height, match_rank
    );
    let covers_strs: Vec<String> = covers
        .iter()
        .map(|CoverRating(r, c)| {
            format!(
                "{}x{} for release {} - {} from {} (diff: {})",
                c.width, c.height, c.artist, c.title, c.provider, r
            )
        })
        .collect();
    loop {
        if Confirm::with_theme(theme)
            .with_prompt("Proceed?")
            .interact()
            .ok()?
        {
            break;
        }

        index = Select::with_theme(theme)
            .items(&covers_strs)
            .default(index)
            .interact()
            .ok()?;
        cover = covers[index].1.clone();
    }
    Some(cover)
}

pub async fn import(path: &PathBuf) -> Result<()> {
    let start = Instant::now();
    let settings = SETTINGS.get().ok_or(eyre!("Could not read settings"))?;
    let theme = DialoguerTheme::default();
    let db = DB.get().ok_or(eyre!("Could not get database"))?;

    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) =
        files.iter().map(TrackFile::open).partition(Result::is_ok);
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
        let id = release.mbid.clone().ok_or(eyre!(
            "The given release doesn't have an ID associated with it, can not fetch specific metadata"
        ))?;
        expanded_releases.push(get(id.as_str()).await?);
    }
    let mut rated_expanded_releases = expanded_releases
        .into_iter()
        .map(|(r, tracks)| {
            let (val, map) = match_tracks(&choice_tracks, &tracks);
            (r, tracks, map, val)
        })
        .collect::<Vec<_>>();
    rated_expanded_releases.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(Ordering::Equal));
    let (mut final_release, mut final_tracks, mut tracks_map, _) = rated_expanded_releases
        .first()
        .ok_or(eyre!("No release available for given tracks"))?
        .clone();
    let mut proceed = false;
    while !proceed {
        (proceed, final_release, final_tracks, tracks_map) = ask(
            &theme,
            &choice_tracks,
            (final_release, final_tracks, tracks_map),
        )
        .await?;
    }

    let covers_by_provider = search_covers(&final_release).await?;
    let covers = rank_covers(covers_by_provider, &final_release);
    let maybe_cover = ask_cover(&theme, covers);
    let mut maybe_picture: Option<Picture> = None;
    let mut final_tracks = tracks_map
        .iter()
        .enumerate()
        .map(|(i, map)| (tracks[i].clone(), final_tracks[*map].clone()))
        .collect::<Vec<_>>();
    for (src, dest) in final_tracks.iter_mut() {
        dest.format = Some(src.format);
        let dest_path = dest.path()?;
        dest.path = Some(dest_path.clone());
    }
    let mut folders = final_tracks
        .iter()
        .map(|(_, t)| {
            Ok(t.path()?
                .parent()
                .ok_or(eyre!("Could not get parent"))?
                .to_path_buf())
        })
        .collect::<Result<Vec<_>>>()?;
    folders.sort();
    folders.dedup();
    for dest in folders.iter() {
        mkdirp(dest)?;
    }
    if let Some(cover) = maybe_cover {
        let (image, mime) = get_cover(cover.url).await?;
        let picture = Picture {
            mime_type: mime,
            picture_type: PictureType::CoverFront,
            description: "Front".to_string(),
            data: image,
        };
        for dest in folders.into_iter() {
            write_picture(&picture, &dest)?;
        }
        maybe_picture = Some(picture)
    } else {
        warn!("No album art found")
    }
    for (src, dest) in final_tracks.iter_mut() {
        debug!("Beofre tagging {:?}", src);
        let path = dest
            .path
            .as_ref()
            .ok_or(eyre!("The track doesn't have an associated path"))?;
        src.duplicate_to(path).wrap_err(eyre!(
            "Could not copy track {:?} to its new location: {:?}",
            src.path,
            path
        ))?;
        if settings.tagging.clear {
            src.clear()
                .wrap_err(eyre!("Could not celar tracks from file: {:?}", path))?;
        }
        src.apply(dest.clone().try_into()?)
            .wrap_err(eyre!("Could not apply new tags to track: {:?}", path))?;
        if let Some(ref picture) = maybe_picture {
            src.set_pictures(vec![picture.clone()])?;
        }
        src.write()
            .wrap_err(eyre!("Could not write tags to track: {:?}", path))?;
        dest.store(db).await?;
        debug!("After tagging {:?}", src);
    }

    info!("Import done, took {:?}", start.elapsed());
    Ok(())
}
