use crate::fetch::{ArtistLike, ReleaseLike};

use levenshtein::levenshtein;
use log::debug;
use std::ops::{Add, Sub};

static TITLE_WEIGHT: f32 = 0.25;
static ARTISTS_WEIGHT: f32 = 0.25;
static TRACKS_WEIGHT: f32 = 0.5;

#[derive(Clone, Debug)]
pub enum AlbumChange {
    TITLE,
    ARTISTS,
}

fn rate_int(original: u64, candidate: u64) -> f32 {
    (1.0 - original.abs_diff(candidate) as f32) / std::cmp::max(original, candidate) as f32
}

fn rate_str(original: String, candidate: String) -> f32 {
    1.0 - (levenshtein(&original, &candidate) as f32
        / std::cmp::max(original.len(), candidate.len()) as f32)
}

// TODO: take mbids and join phrases into consideration
fn rate_artists(original: Vec<Box<dyn ArtistLike>>, candidate: Vec<Box<dyn ArtistLike>>) -> f32 {
    let weight = 1.0 / std::cmp::max(candidate.len(), original.len()) as f32;
    let mut res = 0.0;
    for (i, original_artist) in original.iter().enumerate() {
        if candidate.len() <= i {
            continue;
        }
        res += rate_str(original_artist.name(), candidate[i].name()) * weight;
    }
    if candidate.len() > original.len() {
        // weight newly added values as .5 each element
        res += 0.5 * weight * (candidate.len() - original.len()) as f32;
    }
    res
}

pub fn rate(
    original: Box<dyn ReleaseLike>,
    candidate: Box<dyn ReleaseLike>,
) -> (f32, Vec<AlbumChange>) {
    let mut diffs: Vec<AlbumChange> = vec![];
    // Diff title
    let title_score = rate_str(original.title(), candidate.title()) * TITLE_WEIGHT;
    if title_score != TITLE_WEIGHT {
        diffs.push(AlbumChange::TITLE);
    }
    debug!(
        "Rated album title compatibliity {}: {} -- {}",
        title_score,
        original.title(),
        candidate.title()
    );
    let artists_score = rate_artists(original.artists(), candidate.artists()) * ARTISTS_WEIGHT;
    if artists_score != ARTISTS_WEIGHT {
        diffs.push(AlbumChange::ARTISTS);
    }
    debug!(
        "Rated album artists compatibility {}: {:?} -- {:?}",
        artists_score,
        original.artists(),
        candidate.artists()
    );
    let mut tracks_score = 0.0;
    // TODO: rust currently doesn't allow chaining let Some(..)s
    if let Some(original_tracks) = original.tracks() {
        if let Some(candidate_tracks) = candidate.tracks() {
            let weight = 1.0 / original_tracks.len() as f32;
            for (i, original_track) in original_tracks.iter().enumerate() {
                if candidate_tracks.len() <= i {
                    continue;
                }
                tracks_score += ((rate_str(original_track.title(), candidate_tracks[i].title())
                    * 0.75)
                    + (rate_int(original_track.length(), candidate_tracks[i].length()) * 0.25))
                    * weight;
            }

            // Locally missing tracks
            if original_tracks.len() > candidate_tracks.len() {
                tracks_score -= 0.1 * (candidate_tracks.len() - original_tracks.len()) as f32;
            }
        }
    }
    tracks_score *= TRACKS_WEIGHT;

    (title_score + artists_score + tracks_score, diffs)
}
