use entity::conflict::{
    ARTIST_CONFLICT, ARTIST_CREDIT_CONFLICT, ARTIST_CREDIT_RELEASE_CONFLICT,
    ARTIST_CREDIT_TRACK_CONFLICT, ARTIST_TRACK_RELATION_CONFLICT, MEDIUM_CONFLICT,
    RELEASE_CONFLICT, TRACK_CONFLICT,
};
use entity::full::FullReleaseActive;
use entity::full::{ArtistInfo, FullRelease, FullTrack, FullTrackActive};
use entity::{
    ArtistCreditEntity, ArtistCreditReleaseEntity, ArtistCreditTrackEntity, ArtistEntity,
    ArtistTrackRelationEntity, MediumEntity, ReleaseEntity, TrackEntity,
};
use setting::get_settings;
use tag::{Picture, PictureType};

use dialoguer::{Confirm, Input, Select};
use eyre::{bail, eyre, Context, Result};
use log::{debug, info, warn};
use rayon::prelude::*;
use scan_dir::ScanDir;
use sea_orm::{EntityTrait, TransactionTrait};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::canonicalize;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;
use strfmt::strfmt;
use tag::{strs_from_combination, tags_from_combination, TagKey};

use crate::fetch::cover::{get_cover, search_covers, Cover};
use crate::fetch::{get, search, SearchResult};
use crate::get_database;
use crate::internal::Release;
use crate::rank::{rank_covers, rate_and_match, CoverRating};
use crate::theme::DialoguerTheme;
use crate::track::TrackFile;
use crate::util::{dedup, mkdirp, path_to_str};

struct Job {
    file: TrackFile,
    dest: PathBuf,
    tags: HashMap<TagKey, Vec<String>>,
    cover: Option<Picture>,
}

pub fn track_path(full_release: &FullRelease, full_track: &FullTrack) -> Result<PathBuf> {
    let settings = get_settings()?;
    let vars = strs_from_combination(full_release, full_track)?;
    let release_name = strfmt(settings.release_name.as_str(), &vars)?;
    let track_name = strfmt(settings.track_name.as_str(), &vars)?
        + "."
        + full_track
            .track
            .format
            .ok_or(eyre!("Track is missing file format"))?
            .ext();
    Ok(settings.library.join(release_name).join(track_name))
}

pub fn write_picture<P>(picture: &Picture, root: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let cover_name = &get_settings()?.art.image_name;
    let name = match cover_name {
        Some(n) => n.to_string(),
        None => bail!("Picture write not required"),
    };
    let ext = picture.mime_type.subtype().as_str();
    let filename = PathBuf::from_str((name + "." + ext).as_str())?;
    let path = root.as_ref().join(filename);
    write(path, &picture.data).map_err(|e| eyre!(e))
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

async fn ask(
    theme: &DialoguerTheme,
    original_tracks: &Vec<TrackFile>,
    search_result: SearchResult,
    matching: Vec<usize>,
) -> Result<(bool, SearchResult, Vec<usize>)> {
    let SearchResult(candidate_release, _) = &search_result;
    let FullRelease { release, .. } = &candidate_release;
    info!(
        "Tagging as {} - {} ({})",
        candidate_release.get_joined_artists()?,
        release.title,
        release.id
    );
    let ch = Input::<char>::with_theme(theme)
        .with_prompt("Proceed? [y]es, [n]o, [i]d")
        .interact()
        .map_err(|_| eyre!("Aborted"))?;
    match ch {
        'y' => Ok((true, search_result.clone(), matching)),
        'n' => Err(eyre!("Aborted")),
        'i' => {
            let id: String = Input::with_theme(theme)
                .with_prompt("Enter the MusicBrainz Release ID")
                .interact()
                .map_err(|_| eyre!("Aborted"))?;
            let search_result = get(id.as_str()).await?;
            let (_, tracks_map) = rate_and_match(original_tracks, &search_result);
            Ok((false, search_result, tracks_map))
        }
        v => {
            warn!("Invalid choice: {}", v);
            Ok((false, search_result, matching))
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
    let settings = get_settings()?;
    let theme = DialoguerTheme::default();

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
    let source_release: Release = tracks.clone().into();
    // TODO: reimplement artst & title manual input if they cannot be extracted from the tags
    // see here for the previous implementation:
    // https://codeberg.org/lucat1/tagger/blob/33fa9789ae4e38296edcdfc08270adda6c248529/src/import.rs#L33
    // Decide on what the user interaction should look like before proceeding with the implementation

    info!(
        "Searching for {} - {}...",
        source_release.artists.join(","), // TODO: make "," configurable
        source_release.title
    );
    let search_results = search(&source_release)
        .await
        .wrap_err(eyre!("Error while fetching for album releases"))?;
    info!(
        "Found {} release candidates, ranking...",
        search_results.len()
    );

    let mut expanded_results: Vec<SearchResult> = vec![];
    for result in search_results.into_iter() {
        expanded_results.push(get(result.0.release.id.to_string().as_str()).await?);
    }
    let mut rated_expanded_releases = expanded_results
        .into_iter()
        .map(|search_result| {
            let (val, map) = rate_and_match(&tracks, &search_result);
            (val, search_result, map)
        })
        .collect::<Vec<_>>();
    rated_expanded_releases.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let (_, mut search_result, mut map) = rated_expanded_releases
        .first()
        .ok_or(eyre!("No release available for the given tracks"))?
        .clone();
    let mut proceed = false;
    while !proceed {
        (proceed, search_result, map) = ask(&theme, &tracks, search_result, map).await?;
    }

    let SearchResult(full_release, full_tracks) = search_result;
    let covers_by_provider = search_covers(&full_release).await?;
    let covers = rank_covers(covers_by_provider, &full_release);
    let maybe_cover = ask_cover(&theme, covers);
    let mut start = Instant::now();
    let mut maybe_picture: Option<Picture> = None;
    if let Some(cover) = maybe_cover {
        let (image, mime) = get_cover(cover.url).await?;
        let picture = Picture {
            mime_type: mime,
            picture_type: PictureType::CoverFront,
            description: "Front".to_string(),
            data: image,
        };
        maybe_picture = Some(picture)
    } else {
        warn!("No album art found")
    }
    let mut tasks: Vec<Job> = map
        .iter()
        .enumerate()
        .map(|(i, map)| -> Result<Job> {
            let mut track: FullTrack = full_tracks[*map].clone();
            // let dest_path = release_path.join(full_tracks[*map].0.filename()?.as_os_str());
            let dest_path = format!("tmp/{}.flac", i).into();
            track.track.format = Some(tracks[i].format);
            track.track.path = Some(path_to_str(&dest_path)?);
            Ok(Job {
                file: tracks[i].clone(),
                dest: track_path(&full_release, &track)?,
                tags: tags_from_combination(&full_release, &track)?,
                cover: maybe_picture.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let folders = dedup(
        tasks
            .iter()
            .map(|task| {
                Ok(task
                    .dest
                    .parent()
                    .ok_or(eyre!("Could not get track parent folder"))?
                    .to_path_buf())
            })
            .collect::<Result<Vec<_>>>()?,
    );

    for dest in folders.iter() {
        mkdirp(dest)?;
    }
    if let Some(ref picture) = maybe_picture {
        for dest in folders.into_iter() {
            write_picture(&picture, &dest)?;
        }
    }

    let mut results: Vec<Result<()>> = vec![];
    tasks
        .par_iter_mut()
        .map(|task: &mut Job| -> Result<()> {
            debug!("Beofre tagging {:?}", task.file);
            task.file.duplicate_to(&task.dest).wrap_err(eyre!(
                "Could not copy track {:?} to its new location: {:?}",
                task.file.path,
                path
            ))?;
            if settings.tagging.clear {
                task.file
                    .clear()
                    .wrap_err(eyre!("Could not celar tracks from file: {:?}", path))?;
            }
            task.file
                .apply(task.tags.clone().try_into()?)
                .wrap_err(eyre!("Could not apply new tags to track: {:?}", path))?;
            if let Some(ref picture) = task.cover {
                task.file.set_pictures(vec![picture.clone()])?;
            }
            task.file
                .write()
                .wrap_err(eyre!("Could not write tags to track: {:?}", path))?;

            debug!("After tagging {:?}", task.file);
            Ok(())
        })
        .collect_into_vec(&mut results);
    for res in results.into_iter() {
        if let Err(e) = res {
            bail!(e.wrap_err("While trying to move and apply tags to a track file"));
        }
    }

    info!("Tracks moved. (took {:?})", start.elapsed());
    start = Instant::now();

    let tx = get_database()?.begin().await?;
    let FullReleaseActive {
        release: release_active,
        medium: medium_active,
        artist_credit_release: artist_credit_release_active,
        artist_credit: artist_credit_active,
        artist: artist_active,
        ..
    } = full_release.into();
    ArtistEntity::insert_many(artist_active)
        .on_conflict(ARTIST_CONFLICT.to_owned())
        .exec(&tx)
        .await?;
    ReleaseEntity::insert(release_active)
        .on_conflict(RELEASE_CONFLICT.to_owned())
        .exec(&tx)
        .await?;
    ArtistCreditEntity::insert_many(artist_credit_active)
        .on_conflict(ARTIST_CREDIT_CONFLICT.to_owned())
        .exec(&tx)
        .await?;
    ArtistCreditReleaseEntity::insert_many(artist_credit_release_active)
        .on_conflict(ARTIST_CREDIT_RELEASE_CONFLICT.to_owned())
        .exec(&tx)
        .await?;
    MediumEntity::insert_many(medium_active)
        .on_conflict(MEDIUM_CONFLICT.to_owned())
        .exec(&tx)
        .await?;
    for track in full_tracks.into_iter() {
        let FullTrackActive {
            track: track_active,
            artist_credit_track: artist_credit_track_active,
            artist_credit: artist_credit_active,
            artist_track_relation: artist_track_relation_active,
            artist: artist_active,
        }: FullTrackActive = track.into();
        ArtistEntity::insert_many(artist_active)
            .on_conflict(ARTIST_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        ArtistCreditEntity::insert_many(artist_credit_active)
            .on_conflict(ARTIST_CREDIT_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        TrackEntity::insert(track_active)
            .on_conflict(TRACK_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        ArtistCreditTrackEntity::insert_many(artist_credit_track_active)
            .on_conflict(ARTIST_CREDIT_TRACK_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        ArtistTrackRelationEntity::insert_many(artist_track_relation_active)
            .on_conflict(ARTIST_TRACK_RELATION_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;

    info!("Tracks added. (took {:?})", start.elapsed());
    Ok(())
}
