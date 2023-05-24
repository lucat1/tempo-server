use entity::conflict::{
    ARTIST_CONFLICT, ARTIST_CREDIT_CONFLICT, ARTIST_CREDIT_RELEASE_CONFLICT,
    ARTIST_CREDIT_TRACK_CONFLICT, ARTIST_TRACK_RELATION_CONFLICT, IMAGE_CONFLICT_1,
    IMAGE_CONFLICT_2, IMAGE_RELEASE_CONFLICT, MEDIUM_CONFLICT, RELEASE_CONFLICT, TRACK_CONFLICT,
};
use entity::full::ArtistInfo;
use entity::full::FullReleaseActive;
use entity::full::FullTrackActive;
use entity::{
    ArtistCreditEntity, ArtistCreditReleaseEntity, ArtistCreditTrackEntity, ArtistEntity,
    ArtistTrackRelationEntity, IgnoreNone, Image, ImageActive, ImageEntity, ImageRelease,
    ImageReleaseActive, ImageReleaseEntity, MediumEntity, ReleaseEntity, TrackEntity,
};
use eyre::{eyre, Result, WrapErr};
use rayon::prelude::*;
use scan_dir::ScanDir;
use sea_orm::{EntityTrait, TransactionTrait};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::canonicalize;
use std::fs::write;
use std::path::PathBuf;
use std::str::FromStr;
use strfmt::strfmt;
use uuid::Uuid;

use crate::fetch;
use crate::fetch::cover::get_cover;
use crate::fetch::SearchResult;
use crate::internal;
use crate::rank;
use crate::rank::CoverRating;
use crate::track::TrackFile;
use base::{
    database::get_database,
    setting::{get_settings, Library},
    util::{dedup, mkdirp, path_to_str},
};
use tag::{
    sanitize_map, tag_to_string_map, tags_from_combination, tags_from_full_release, Picture,
    PictureType, TagKey,
};

#[derive(Serialize, Clone)]
pub struct RatedSearchResult {
    rating: i64,
    pub search_result: fetch::SearchResult,
    mapping: Vec<usize>,
}

#[derive(Serialize, Clone)]
pub struct Import {
    #[serde(skip_serializing)]
    track_files: Vec<TrackFile>,

    release: internal::Release,
    tracks: Vec<internal::Track>,

    pub search_results: Vec<RatedSearchResult>,
    pub covers: Vec<CoverRating>,
    pub selected: (Uuid, Option<usize>),
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

pub async fn all_tracks(library: &Library, path: &PathBuf) -> Result<Vec<TrackFile>> {
    let files = all_files(&canonicalize(path)?)?;
    let (tracks, errors): (Vec<_>, Vec<_>) = files
        .iter()
        .map(|f| TrackFile::open(library, f))
        .partition(Result::is_ok);
    let tracks: Vec<_> = tracks.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();
    tracing::info! {
        tracks=%tracks.len(),
        errors=%errors.len(),
        "Found tracks, with files ignored due to errors"
    };
    if !errors.is_empty() {
        errors
            .iter()
            .for_each(|error| tracing::trace! {%error, "Error while importing file"});
    }
    Ok(tracks)
}

pub async fn begin(path: &PathBuf) -> Result<Import> {
    let settings = get_settings()?;
    let library = &settings.library;

    tracing::info! {?path, library = library.name, "Importing folder for the given library"};
    let tracks = all_tracks(library, path).await?;
    if tracks.is_empty() {
        return Err(eyre!("No tracks to import were found"));
    }

    let source_release: internal::Release = tracks.clone().into();
    let source_tracks: Vec<internal::Track> = tracks.iter().map(|t| t.clone().into()).collect();
    tracing::info! {artists = source_release.artists.join(", "), title = source_release.title, "Searching for"};
    let compressed_search_results = fetch::search(library, &source_release)
        .await
        .wrap_err(eyre!("Error while fetching for album releases"))?;
    let mut search_results: Vec<fetch::SearchResult> = vec![];
    for result in compressed_search_results.into_iter() {
        search_results.push(fetch::get(library, result.0.release.id.to_string().as_str()).await?);
    }
    let mut rated_search_results = search_results
        .into_iter()
        .map(|search_result| {
            let rank::Rating(rating, mapping) = rank::rate_and_match(&tracks, &search_result);
            RatedSearchResult {
                rating,
                search_result,
                mapping,
            }
        })
        .collect::<Vec<_>>();
    rated_search_results.sort_by(|a, b| a.rating.partial_cmp(&b.rating).unwrap_or(Ordering::Equal));
    let fetch::SearchResult(full_release, _) = rated_search_results
        .first()
        .map(|r| r.search_result.clone())
        .ok_or(eyre!("No results found"))?;
    let covers_by_provider = fetch::cover::search(library, &full_release).await?;
    let covers = rank::rank_covers(library, covers_by_provider, &full_release);
    let selected = (
        full_release.release.id,
        if !covers.is_empty() { Some(0) } else { None },
    );
    Ok(Import {
        track_files: tracks,
        release: source_release,
        tracks: source_tracks,

        search_results: rated_search_results,
        covers,
        selected,
    })
}

struct Job {
    file: TrackFile,
    dest: PathBuf,
    tags: HashMap<TagKey, Vec<String>>,
    cover: Option<Picture>,
}

pub async fn run(import: Import) -> Result<()> {
    let settings = get_settings()?;
    let library = &settings.library;

    let RatedSearchResult {
        search_result: SearchResult(mut full_release, mut full_tracks),
        mapping,
        ..
    } = import
        .search_results
        .iter()
        .find(|sr| sr.search_result.0.release.id == import.selected.0)
        .ok_or(eyre!("Invalid selected release id"))?
        .clone();
    tracing::info! {
        joined_artists=full_release.get_joined_artists()?,
        title=full_release.release.title,
        "Adding release to the library",
    };

    let mut picture = None;
    if let Some(selected_cover) = import.selected.1 {
        let cover = import
            .covers
            .get(selected_cover)
            .ok_or(eyre!("Invalid selected cover"))?;
        let (data, (width, height), mime) = get_cover(library, &cover.1).await?;
        let size = data.len() as u32;
        let pic = Picture {
            mime_type: mime,
            picture_type: PictureType::CoverFront,
            description: "Front cover".to_string(),
            data,
        };
        let image = Image {
            role: pic.picture_type.to_string(),
            format: library.art.format,
            description: None,
            width,
            height,
            size,
            // will be replaced afterwards
            id: "".to_string(),
            path: "".to_string(),
        };
        picture = Some((pic, image));
    }

    let release_root = library.path.join(PathBuf::from_str(
        strfmt(
            library.release_name.as_str(),
            &sanitize_map(tag_to_string_map(&tags_from_full_release(&full_release)?)),
        )?
        .as_str(),
    )?);
    full_release.release.path = Some(path_to_str(&release_root)?);
    let mut tasks: Vec<Job> = mapping
        .iter()
        .enumerate()
        .map(|(from, to)| -> Result<Job> {
            let full_track = &mut full_tracks[*to];
            let tags = tags_from_combination(&full_release, full_track)?;
            let vars = tag_to_string_map(&tags);
            let track_name = strfmt(library.track_name.as_str(), &sanitize_map(vars.clone()))?;
            let dest = release_root.join(PathBuf::from_str(
                format!(
                    "{}.{}",
                    track_name.as_str(),
                    import.track_files[from].format.ext()
                )
                .as_str(),
            )?);
            full_track.track.format = Some(import.track_files[from].format);
            full_track.track.path = Some(path_to_str(&dest)?);
            Ok(Job {
                file: import.track_files[from].clone(),
                cover: picture.clone().map(|(pic, _)| pic),
                tags,
                dest,
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
        mkdirp(dest).wrap_err(eyre!("Could not create folder {:?} for release", dest))?;
    }
    tasks
        .par_iter_mut()
        .try_for_each(|task: &mut Job| -> Result<()> {
            task.file.duplicate_to(library, &task.dest).wrap_err(eyre!(
                "Could not copy track {:?} to its new location: {:?}",
                task.file.path,
                task.dest
            ))?;
            if library.tagging.clear {
                task.file
                    .clear()
                    .wrap_err(eyre!("Could not celar tracks from file: {:?}", task.dest))?;
            }
            task.file
                .apply(task.tags.clone())
                .wrap_err(eyre!("Could not apply new tags to track: {:?}", task.dest))?;
            task.file
                .write()
                .wrap_err(eyre!("Could not write tags to track: {:?}", task.dest))?;
            if let Some(ref picture) = task.cover {
                task.file
                    .set_pictures(vec![picture.clone()])
                    .wrap_err(eyre!("Could not add picture tag to file: {:?}", task.dest))?;
            }
            Ok(())
        })?;

    let tx = get_database()?.begin().await?;
    let FullReleaseActive {
        release: release_active,
        medium: medium_active,
        artist_credit_release: artist_credit_release_active,
        artist_credit: artist_credit_active,
        artist: artist_active,
        ..
    } = full_release.to_owned().into();
    ArtistEntity::insert_many(artist_active)
        .on_conflict(ARTIST_CONFLICT.to_owned())
        .exec(&tx)
        .await
        .ignore_none()?;
    ReleaseEntity::insert(release_active)
        .on_conflict(RELEASE_CONFLICT.to_owned())
        .exec(&tx)
        .await
        .ignore_none()?;
    ArtistCreditEntity::insert_many(artist_credit_active)
        .on_conflict(ARTIST_CREDIT_CONFLICT.to_owned())
        .exec(&tx)
        .await
        .ignore_none()?;
    ArtistCreditReleaseEntity::insert_many(artist_credit_release_active)
        .on_conflict(ARTIST_CREDIT_RELEASE_CONFLICT.to_owned())
        .exec(&tx)
        .await
        .ignore_none()?;
    MediumEntity::insert_many(medium_active)
        .on_conflict(MEDIUM_CONFLICT.to_owned())
        .exec(&tx)
        .await
        .ignore_none()?;

    if let Some((pic, image)) = picture.as_mut() {
        let cover_name = library.art.image_name.clone();
        let ext = pic.mime_type.subtype().as_str();
        let filename = PathBuf::from_str((cover_name + "." + ext).as_str())?;
        let path = release_root.join(filename);

        image.path = path_to_str(&path)?;
        image.id = sha256::digest(image.path.to_owned());
        write(path, &pic.data).map_err(|e| eyre!(e))?;
        let image_release = ImageRelease {
            image_id: image.id.to_owned(),
            release_id: full_release.release.id,
        };
        let active_image: ImageActive = image.clone().into();
        let active_image_release: ImageReleaseActive = image_release.into();

        ImageEntity::insert(active_image)
            .on_conflict(IMAGE_CONFLICT_1.to_owned())
            .on_conflict(IMAGE_CONFLICT_2.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;

        ImageReleaseEntity::insert(active_image_release)
            .on_conflict(IMAGE_RELEASE_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
    }

    for track in full_tracks.iter() {
        let FullTrackActive {
            track: track_active,
            artist_credit_track: artist_credit_track_active,
            artist_credit: artist_credit_active,
            artist_track_relation: artist_track_relation_active,
            artist: artist_active,
        }: FullTrackActive = track.to_owned().into();
        ArtistEntity::insert_many(artist_active)
            .on_conflict(ARTIST_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        ArtistCreditEntity::insert_many(artist_credit_active)
            .on_conflict(ARTIST_CREDIT_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        TrackEntity::insert(track_active)
            .on_conflict(TRACK_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        ArtistCreditTrackEntity::insert_many(artist_credit_track_active)
            .on_conflict(ARTIST_CREDIT_TRACK_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        ArtistTrackRelationEntity::insert_many(artist_track_relation_active)
            .on_conflict(ARTIST_TRACK_RELATION_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
    }
    tx.commit().await?;
    Ok(())
}
