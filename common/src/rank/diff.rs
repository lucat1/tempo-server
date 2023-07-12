use levenshtein::levenshtein;
use std::ops::Sub;

use entity::{InternalRelease, InternalTrack};

static TRACK_TITLE_FACTOR: usize = 5000;
static TRACK_LENGTH_FACTOR: u32 = 300;
static TRACK_DISC_FACTOR: u32 = 100;
static TRACK_NUMBER_FACTOR: u32 = 200;

static RELEASE_TITLE_FACTOR: usize = 1000;
static RELEASE_MEDIA_FACTOR: usize = 10;
static RELEASE_DISCS_FACTOR: u32 = 100;
static RELEASE_TRACKS_FACTOR: u32 = 1000;
static RELEASE_COUNTRY_FACTOR: usize = 5;
static RELEASE_LABEL_FACTOR: usize = 5;
static RELEASE_RELEASE_TYPE_FACTOR: usize = 50;
static RELEASE_YEAR_FACTOR: i32 = 100;
static RELEASE_MONTH_FACTOR: i64 = 50;
static RELEASE_DAY_FACTOR: i64 = 10;
static RELEASE_ORIGINAL_YEAR_FACTOR: i32 = 100;
static RELEASE_ORIGINAL_MONTH_FACTOR: i64 = 50;
static RELEASE_ORIGINAL_DAY_FACTOR: i64 = 10;

pub trait Diff {
    fn diff(&self, other: &Self) -> i64;
}

fn if_both<T, R>(a: Option<T>, b: Option<T>, then: impl Fn(T, T) -> R) -> Option<R> {
    if let Some(a_val) = a {
        if let Some(b_val) = b {
            return Some(then(a_val, b_val));
        }
    }
    None
}

impl Diff for InternalTrack {
    fn diff(&self, other: &Self) -> i64 {
        // TODO: diff artists
        ((levenshtein(self.title.as_str(), other.title.as_str()) * TRACK_TITLE_FACTOR) as i64)
            + if_both(self.length, other.length, |l1, l2| {
                (l1.abs_diff(l2) * TRACK_LENGTH_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.disc, other.disc, |n1, n2| {
                (n1.abs_diff(n2) * TRACK_DISC_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.number, other.number, |n1, n2| {
                (n1.abs_diff(n2) * TRACK_NUMBER_FACTOR) as i64
            })
            .unwrap_or_default()
    }
}

impl Diff for InternalRelease {
    fn diff(&self, other: &Self) -> i64 {
        // TODO: diff artists
        ((levenshtein(self.title.as_str(), other.title.as_str()) * RELEASE_TITLE_FACTOR) as i64)
            + if_both(self.media.as_ref(), other.media.as_ref(), |l1, l2| {
                (levenshtein(l1.as_str(), l2.as_str()) * RELEASE_MEDIA_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.discs, other.discs, |n1, n2| {
                (n1.abs_diff(n2) * RELEASE_DISCS_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.tracks, other.tracks, |n1, n2| {
                (n1.abs_diff(n2) * RELEASE_TRACKS_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.country.as_ref(), other.country.as_ref(), |c1, c2| {
                (levenshtein(c1.as_str(), c2.as_str()) * RELEASE_COUNTRY_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.label.as_ref(), other.label.as_ref(), |l1, l2| {
                (levenshtein(l1.as_str(), l2.as_str()) * RELEASE_LABEL_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(
                self.release_type.as_ref(),
                other.release_type.as_ref(),
                |r1, r2| {
                    (levenshtein(r1.as_str(), r2.as_str()) * RELEASE_RELEASE_TYPE_FACTOR) as i64
                },
            )
            .unwrap_or_default()
            + if_both(self.year, other.year, |d1, d2| {
                (d1.sub(d2).abs() * RELEASE_YEAR_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.month, other.month, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_MONTH_FACTOR
            })
            .unwrap_or_default()
            + if_both(self.day, other.day, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_DAY_FACTOR
            })
            .unwrap_or_default()
            + if_both(self.original_year, other.original_year, |d1, d2| {
                (d1.sub(d2).abs() * RELEASE_ORIGINAL_YEAR_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.original_month, other.original_month, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_ORIGINAL_MONTH_FACTOR
            })
            .unwrap_or_default()
            + if_both(self.original_day, other.original_day, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_ORIGINAL_DAY_FACTOR
            })
            .unwrap_or_default()
    }
}
