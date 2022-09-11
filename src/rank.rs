use crate::models::Track;

use levenshtein::levenshtein;
use log::{debug, trace};
use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

// static TITLE_WEIGHT: f32 = 0.25;
// static ARTISTS_WEIGHT: f32 = 0.25;
// static TRACKS_WEIGHT: f32 = 0.5;
//
// #[derive(Clone, Debug)]
// pub enum AlbumChange {
//     TITLE,
//     ARTISTS,
// }
//
// fn rate_int(original: u64, candidate: u64) -> f32 {
//     (1.0 - original.abs_diff(candidate) as f32) / std::cmp::max(original, candidate) as f32
// }
//
// fn rate_str(original: &str, candidate: &str) -> f32 {
//     1.0 - (levenshtein(original, candidate) as f32
//         / std::cmp::max(original.len(), candidate.len()) as f32)
// }
//
// // TODO: take mbids and join phrases into consideration
// fn rate_artists(original: Vec<Artist>, candidate: Vec<Artist>) -> f32 {
//     // TODO: don't expect accurate sorting, use monkers here too
//     let weight = 1.0 / std::cmp::max(candidate.len(), original.len()) as f32;
//     let mut res = 0.0;
//     for (i, original_artist) in original.iter().enumerate() {
//         if candidate.len() <= i {
//             continue;
//         }
//         res += rate_str(original_artist.name.as_str(), candidate[i].name.as_str()) * weight;
//     }
//     if candidate.len() > original.len() {
//         // weight newly added values as .5 each element
//         res += 0.5 * weight * (candidate.len() - original.len()) as f32;
//     }
//     res
// }

fn if_both<T, R>(a: Option<T>, b: Option<T>, then: impl Fn(T, T) -> R) -> Option<R> {
    if let Some(a_val) = a {
        if let Some(b_val) = b {
            return Some(then(a_val, b_val));
        }
    }
    None
}

fn if_both_or_default<T: Default, R>(a: Option<T>, b: Option<T>, then: impl Fn(T, T) -> R) -> R {
    let a_val = match a {
        Some(a_val) => a_val,
        None => T::default(),
    };
    let b_val = match b {
        Some(b_val) => b_val,
        None => T::default(),
    };
    then(a_val, b_val)
}

pub fn match_tracks(
    original_tracks: &Vec<Track>,
    candidate_tracks: &Vec<Track>,
) -> (i64, Vec<usize>) {
    let rows = original_tracks.len();
    let mut columns = candidate_tracks.len();
    let mut matrix_vec = vec![];
    for original_track in original_tracks.iter() {
        for candidate_track in candidate_tracks.iter() {
            let distance = ((levenshtein(
                original_track.title.as_str(),
                candidate_track.title.as_str(),
            ) * 1000) as i64)
                + if_both(
                    original_track.length,
                    candidate_track.length,
                    |len1, len2| len1.as_secs().abs_diff(len2.as_secs()) as i64,
                )
                .unwrap_or(0) // TODO: add weight for this
                + if_both_or_default(original_track.mbid.clone(), candidate_track.mbid.clone(), |mbid1, mbid2| {
                    levenshtein(mbid1.as_str(), mbid2.as_str()) as i64
                })
                + if_both_or_default(original_track.number, candidate_track.number, |n1, n2| {
                    n1.abs_diff(n2) as i64
                });

            trace!(
                "Rated track compatibility {}: {:?} -- {:?}",
                distance,
                original_track,
                candidate_track
            );
            matrix_vec.push(distance);
        }
    }
    if matrix_vec.len() == 0 {
        return (0, vec![]);
    }
    debug!("kuhn_munkers matrix is {}x{}", rows, columns);
    if rows > columns {
        let max = match matrix_vec.iter().max() {
            Some(v) => *v,
            None => i64::MAX / (rows as i64),
        } + 1;
        for _ in 0..((rows - columns) * rows) {
            matrix_vec.push(max);
        }
        columns = rows
    }
    let matrix = Matrix::from_vec(rows, columns, matrix_vec);
    kuhn_munkres_min(&matrix)
}
