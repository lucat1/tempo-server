mod convert;
mod cover;
mod diff;

use crate::internal::{Release, Track};
pub use cover::{rank_covers, CoverRating};
pub use diff::Diff;
use log::debug;
use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

use crate::track::TrackFile;

pub fn rate_and_match(
    tracks: &Vec<TrackFile>,
    full_release: &entity::FullRelease,
    full_tracks: &entity::FullTrack,
) -> (i64, Vec<usize>) {
    let release: Release = tracks.clone().into();
    let candidate_release: Release = full_release.into();

    let rows = tracks.len();
    let mut columns = full_tracks.len();
    let mut matrix_vec = vec![];

    for original_track in tracks.iter() {
        for candidate_track in full_tracks.iter() {
            matrix_vec.push(original_track.into().diff(&candidate_track.into()));
        }
    }
    if matrix_vec.is_empty() {
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
    let (val, map) = kuhn_munkres_min(&matrix);
    (val + release.diff(&candidate_release) as i64, map)
}
