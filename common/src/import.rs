use base::setting::get_settings;
use base::setting::Library;
use base::util::path_to_str;
use eyre::{eyre, Result, WrapErr};
use log::debug;
use scan_dir::ScanDir;
use serde::Serialize;
use std::cmp::Ordering;
use std::fs::canonicalize;
use std::path::PathBuf;
use uuid::Uuid;

use crate::fetch;
use crate::internal;
use crate::rank;
use crate::rank::CoverRating;
use crate::track::TrackFile;

#[derive(Serialize, Clone)]
pub struct RatedSearchResult {
    rating: i64,
    search_result: fetch::SearchResult,
    mapping: Vec<usize>,
}

#[derive(Serialize, Clone)]
pub struct Import {
    #[serde(skip_serializing)]
    abs_path: PathBuf,
    library: usize,

    #[serde(skip_serializing)]
    track_files: Vec<TrackFile>,

    release: internal::Release,
    tracks: Vec<internal::Track>,

    search_results: Vec<RatedSearchResult>,
    covers: Vec<CoverRating>,
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
    debug!("Found {} tracks, {} errors", tracks.len(), errors.len());
    if !errors.is_empty() {
        errors
            .iter()
            .for_each(|e| debug!("Error while importing file:{}", e));
    }
    Ok(tracks)
}

pub async fn begin(lib: usize, path: &PathBuf) -> Result<Import> {
    let settings = get_settings()?;
    let library = if settings.libraries.len() <= lib {
        Err(eyre!("Invalid library id"))
    } else {
        Ok(&settings.libraries[lib])
    }?;

    let tracks = all_tracks(&library, path).await?;
    if tracks.is_empty() {
        return Err(eyre!("No tracks to import were found"));
    }

    let source_release: internal::Release = tracks.clone().into();
    let source_tracks: Vec<internal::Track> = tracks.iter().map(|t| t.clone().into()).collect();
    let compressed_search_results = fetch::search(&library, &source_release)
        .await
        .wrap_err(eyre!("Error while fetching for album releases"))?;
    let mut search_results: Vec<fetch::SearchResult> = vec![];
    for result in compressed_search_results.into_iter() {
        search_results.push(fetch::get(&library, result.0.release.id.to_string().as_str()).await?);
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
        if covers.len() > 0 { Some(0) } else { None },
    );
    Ok(Import {
        abs_path: path.to_path_buf(),
        library: lib,

        track_files: tracks,

        release: source_release,
        tracks: source_tracks,
        search_results: rated_search_results,
        covers,
        selected,
    })
}
