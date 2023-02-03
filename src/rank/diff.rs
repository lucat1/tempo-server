use crate::rank::{Release, Track};
use levenshtein::levenshtein;

static TRACK_TITLE_FACTOR: usize = 1000;
static TRACK_LENGTH_FACTOR: usize = 300;
static TRACK_DISC_FACTOR: usize = 100;
static TRACK_NUMBER_FACTOR: usize = 200;

static RELEASE_TITLE_FACTOR: usize = 1000;
static RELEASE_MEDIA_FACTOR: usize = 10;
static RELEASE_DISCS_FACTOR: usize = 100;
static RELEASE_TRACKS_FACTOR: usize = 1000;
static RELEASE_COUNTRY_FACTOR: usize = 5;
static RELEASE_LABEL_FACTOR: usize = 5;
static RELEASE_RELEASE_TYPE_FACTOR: usize = 50;
static RELEASE_DATE_FACTOR: usize = 100;
static RELEASE_ORIGINAL_DATE_FACTOR: usize = 100;

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

impl Diff for Track {
    fn diff(&self, other: &Self) -> i64 {
        // TODO: diff artists
        ((levenshtein(self.title.as_str(), other.title.as_str()) * TRACK_TITLE_FACTOR) as i64)
            + if_both(self.length, other.length, |l1, l2| {
                (l1.abs_diff(l2) * TRACK_LENGTH_FACTOR) as i64
            })
            + if_both(self.disc, other.disc, |n1, n2| {
                (n1.abs_diff(n2) * TRACK_DISC_FACTOR) as i64
            })
            + if_both(self.number, other.number, |n1, n2| {
                (n1.abs_diff(n2) * TRACK_NUMBER_FACTOR) as i64
            })
    }
}

impl Diff for Release {
    fn diff(&self, other: &Self) -> i64 {
        // TODO: diff artists
        ((levenshtein(self.title.as_str(), other.title.as_str()) * RELEASE_TITLE_FACTOR) as i64)
            + if_both(self.media, other.media, |l1, l2| {
                (levenshtein(l1.as_str(), l2.as_str()) * RELEASE_MEDIA_FACTOR) as i64
            })
            + if_both(self.discs, other.discs, |n1, n2| {
                (n1.abs_diff(n2) * RELEASE_DISCS_FACTOR) as i64
            })
            + if_both(self.tracks, other.tracks, |n1, n2| {
                (n1.abs_diff(n2) * RELEASE_TRACKS_FACTOR) as i64
            })
            + if_both(self.country, other.country, |c1, c2| {
                (levenshtein(c1.as_str(), c2.as_str()) * RELEASE_COUNTRY_FACTOR) as i64
            })
            + if_both(self.label, other.label, |l1, l2| {
                (levenshtein(l1.as_str(), l2.as_str()) * RELEASE_LABEL_FACTOR) as i64
            })
            + if_both(self.release_type, other.release_type, |r1, r2| {
                (levenshtein(r1.as_str(), r2.as_str()) * RELEASE_RELEASE_TYPE_FACTOR) as i64
            })
            + if_both(self.date, other.date, |d1, d2| {
                (d1.signed_duration_since(d2).num_days() * RELEASE_DATE_FACTOR) as i64
            })
            + if_both(self.original_date, other.original_date, |d1, d2| {
                (d1.signed_duration_since(d2).num_days() * RELEASE_ORIGINAL_DATE_FACTOR) as i64
            })
    }
}
