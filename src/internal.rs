use chrono::NaiveDate;
use entity::{FullRelease, FullTrack};

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";
pub const UNKNOWN_TITLE: &str = "(unkown title)";

pub struct Track {
    pub title: String,
    pub artists: Vec<String>,
    pub length: Option<u64>,
    pub disc: Option<u64>,
    pub number: Option<u64>,
}

pub struct Release {
    pub title: String,
    pub artists: Vec<String>,
    pub media: Option<String>,
    pub discs: Option<u64>,
    pub tracks: Option<u64>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub release_type: Option<String>,
    pub date: Option<NaiveDate>,
    pub original_date: Option<NaiveDate>,
}

impl From<FullRelease> for Release {
    fn from(full_release: entity::FullRelease) -> Self {
        let FullRelease {
            release,
            medium,
            artist,
            ..
        } = full_release;
        Release {
            title: release.title,
            artists: artist.into_iter().map(|a| a.name.clone()).collect(),
            discs: Some(medium.len() as u64),
            media: medium.first().as_ref().and_then(|m| m.format.clone()),
            tracks: None, // TODO: consider adding a track count in the media structure
            country: release.country,
            label: release.label,
            release_type: release.release_type,
            date: release.date,
            original_date: release.original_date,
        }
    }
}

impl From<FullTrack> for Track {
    fn from(full_track: entity::FullTrack) -> Self {
        let FullTrack { track, artist, .. } = full_track;
        Track {
            title: track.title,
            artists: artist.into_iter().map(|a| a.name.clone()).collect(),
            length: Some(track.length),
            disc: None, // TODO: see above
            number: Some(track.number),
        }
    }
}
