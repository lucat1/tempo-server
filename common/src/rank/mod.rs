mod cover;
mod diff;

pub use cover::{rank_covers, CoverRating};
pub use diff::Diff;
use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

use crate::fetch::SearchResult;
use entity::{InternalRelease, InternalTrack};

pub struct Rating(pub i64, pub Vec<usize>);

pub fn rate_and_match(
    (release, tracks): (InternalRelease, Vec<InternalTrack>),
    result: &SearchResult,
) -> Rating {
    let SearchResult(full_release, full_tracks) = result;
    let candidate_release: InternalRelease = full_release.clone().into();

    let rows = tracks.len();
    let mut columns = full_tracks.len();
    let mut matrix_vec = vec![];

    for original_track in tracks.iter() {
        for candidate_track in full_tracks.iter() {
            matrix_vec.push(original_track.diff(&candidate_track.clone().into()));
        }
    }
    if matrix_vec.is_empty() {
        return Rating(0, vec![]);
    }
    tracing::debug! {%rows, %columns, "The kuhn_munkers matrix has size"};
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
    let (val, map) = kuhn_munkres_min(&matrix);
    let value = val + release.diff(&candidate_release);
    tracing::debug! {
        artists = ?candidate_release.artists,
        title = %candidate_release.title,
        %value,
        "Rating value for"
    };
    Rating(value, map)
}
