use crate::fetch::structures::Cover;
use crate::models::{Track, Release, Artists};

use levenshtein::levenshtein;
use log::{debug, trace};
use eyre::{eyre, Result};
use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

static TRACK_TITLE_FACTOR: usize = 1000;
static RELEASE_TITLE_FACTOR: usize = 10000;

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
            ) * TRACK_TITLE_FACTOR) as i64)
                + if_both(
                    original_track.length,
                    candidate_track.length,
                    |len1, len2| len1.as_secs().abs_diff(len2.as_secs()) as i64,
                )
                .unwrap_or(0) // TODO: add weight for this
                + if_both_or_default(original_track.mbid.clone(), candidate_track.mbid.clone(), |mbid1, mbid2| {
                    levenshtein(mbid1.as_str(), mbid2.as_str()) as i64
                })
                + if_both_or_default(original_track.disc, candidate_track.disc, |n1, n2| {
                    n1.abs_diff(n2) as i64
                }) 
                + if_both_or_default(original_track.disc_mbid.clone(), candidate_track.disc_mbid.clone(), |mbid1, mbid2| {
                    levenshtein(mbid1.as_str(), mbid2.as_str()) as i64
                })
                + if_both_or_default(original_track.number, candidate_track.number, |n1, n2| {
                    n1.abs_diff(n2) as i64
                })
                + if_both(original_track.release.clone(), candidate_track.release.clone(), |r1, r2| {
                        (levenshtein(r1.title.as_str(), r2.title.as_str())*RELEASE_TITLE_FACTOR) as i64
                        + if_both_or_default(r1.mbid.clone(), r2.mbid.clone(), |mbid1, mbid2| {
                            levenshtein(mbid1.as_str(), mbid2.as_str()) as i64
                        })
                        + if_both_or_default(r1.asin.clone(), r2.asin.clone(), |asin1, asin2| {
                            levenshtein(asin1.as_str(), asin2.as_str()) as i64
                        })
                        + if_both_or_default(r1.discs, r2.discs, |discs1, discs2| {
                            discs1.abs_diff(discs2) as i64
                        })
                        + if_both_or_default(r1.media.clone(), r2.media.clone(), |media1, media2| {
                            levenshtein(media1.as_str(), media2.as_str()) as i64
                        })
                        + if_both_or_default(r1.tracks, r2.tracks, |tracks1, tracks2| {
                            tracks1.abs_diff(tracks2) as i64
                        }) * 100
                        + if_both_or_default(r1.country.clone(), r2.country.clone(), |country1, country2| {
                            levenshtein(country1.as_str(), country2.as_str()) as i64
                        })
                        + if_both_or_default(r1.status.clone(), r2.status.clone(), |status1, status2| {
                            levenshtein(status1.as_str(), status2.as_str()) as i64
                        })
                        + if_both_or_default(r1.date, r2.date, |date1, date2| {
                            date1.signed_duration_since(date2).num_days()
                        })
                        + if_both_or_default(r1.original_date, r2.original_date, |date1, date2| {
                            date1.signed_duration_since(date2).num_days()
                        })
                        + if_both_or_default(r1.script.clone(), r2.script.clone(), |script1, script2| {
                            levenshtein(script1.as_str(), script2.as_str()) as i64
                        })
                }).unwrap_or(0);

            trace!("Rated track compatibility {}", distance);
            matrix_vec.push(distance);
        }
    }
    if matrix_vec.is_empty(){
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

pub fn rank_covers(covers_by_provider: &mut [Vec<Cover>], release: &Release) -> Result<Cover> {
    for covers in covers_by_provider.iter_mut() {
        let mut rank_to_index = covers.iter().enumerate().map(|(i, c)| (levenshtein(c.title.as_str(), release.title.as_str()) + levenshtein(c.artist.as_str(), release.artists.joined().as_str()), i)).collect::<Vec<_>>();
        rank_to_index.sort_by(|d1,d2| d1.0.cmp(&d2.0));
        if let Some(c) = rank_to_index.first() {
                return Ok(covers[c.1].clone());
        }
    }
    Err(eyre!("No cover art found"))
}
