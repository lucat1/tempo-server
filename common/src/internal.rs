use chrono::NaiveDate;
use entity::full::{FullRelease, FullTrack};
use itertools::Itertools;
use log::warn;
use serde::Serialize;
use tag::TagKey;

use crate::track::TrackFile;
use base::util::{dedup, maybe_date};

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";
pub const UNKNOWN_TITLE: &str = "(unkown title)";

#[derive(Serialize, Clone)]
pub struct Track {
    pub title: String,
    pub artists: Vec<String>,
    pub length: Option<u32>,
    pub disc: Option<u32>,
    pub number: Option<u32>,
}

#[derive(Serialize, Clone)]
pub struct Release {
    pub title: String,
    pub artists: Vec<String>,
    pub media: Option<String>,
    pub discs: Option<u32>,
    pub tracks: Option<u32>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub release_type: Option<String>,
    pub date: Option<NaiveDate>,
    pub original_date: Option<NaiveDate>,
}

impl From<FullRelease> for Release {
    fn from(full_release: entity::full::FullRelease) -> Self {
        let FullRelease {
            release,
            medium,
            artist,
            ..
        } = full_release;
        Release {
            title: release.title,
            artists: artist.into_iter().map(|a| a.name).collect(),
            discs: Some(medium.len() as u32),
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
    fn from(full_track: entity::full::FullTrack) -> Self {
        let FullTrack { track, artist, .. } = full_track;
        Track {
            title: track.title,
            artists: artist.into_iter().map(|a| a.name).collect(),
            length: Some(track.length),
            disc: None, // TODO: see above
            number: Some(track.number),
        }
    }
}

fn artists_with_name(name: String, sep: Option<String>) -> Vec<String> {
    match sep {
        Some(ref s) => name.split(s.as_str()).map(|s| s.to_string()).collect_vec(),
        None => vec![name],
    }
}

fn artists_from_tag(tracks: &[TrackFile], tag: TagKey) -> Vec<String> {
    let separator = match tracks.first() {
        Some(t) => t.tag.separator(),
        None => return vec![],
    };
    dedup(tracks.iter().flat_map(|t| t.get_tag(tag)).collect())
        .iter()
        .flat_map(|name| artists_with_name(name.to_string(), separator.clone()))
        .collect()
}

fn first_tag(tracks: &[TrackFile], tag: TagKey) -> Option<String> {
    let options = dedup(tracks.iter().flat_map(|t| t.get_tag(tag)).collect());
    if options.len() > 1 {
        warn!(
            "Multiple ({}) unique tag values for {:?} in the given tracks ({})",
            options.len(),
            tag,
            // TODO
            options.join(", ")
        );
    }
    options.first().map(|f| f.to_string())
}

impl From<TrackFile> for Track {
    fn from(file: TrackFile) -> Self {
        let file_singleton = vec![file];
        Track {
            title: first_tag(&file_singleton, TagKey::TrackTitle)
                .unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists: artists_from_tag(&file_singleton, TagKey::Artists),
            length: first_tag(&file_singleton, TagKey::Duration)
                .and_then(|d| d.parse::<u32>().ok()),
            disc: first_tag(&file_singleton, TagKey::DiscNumber)
                .and_then(|d| d.parse::<u32>().ok()),
            number: first_tag(&file_singleton, TagKey::TrackNumber)
                .and_then(|d| d.parse::<u32>().ok()),
        }
    }
}

impl From<Vec<TrackFile>> for Release {
    fn from(tracks: Vec<TrackFile>) -> Self {
        let artists = if first_tag(&tracks, TagKey::AlbumArtist).is_some() {
            // Use the AlbumArtist to search if we have one available
            artists_from_tag(&tracks, TagKey::AlbumArtist)
        } else {
            // Otherwise use the Artist tag
            let mut v1 = artists_from_tag(&tracks, TagKey::Artist);
            let mut v2 = artists_from_tag(&tracks, TagKey::Artists);
            v1.append(&mut v2);
            v1
        };

        Release {
            title: first_tag(&tracks, TagKey::Album).unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists,
            discs: first_tag(&tracks, TagKey::TotalDiscs).and_then(|d| d.parse::<u32>().ok()),
            media: first_tag(&tracks, TagKey::Media),
            tracks: first_tag(&tracks, TagKey::TotalTracks).and_then(|d| d.parse::<u32>().ok()),
            country: first_tag(&tracks, TagKey::ReleaseCountry),
            label: first_tag(&tracks, TagKey::RecordLabel),
            release_type: first_tag(&tracks, TagKey::ReleaseType),
            date: maybe_date(
                first_tag(&tracks, TagKey::ReleaseDate)
                    .or_else(|| first_tag(&tracks, TagKey::ReleaseYear)),
            ),
            original_date: maybe_date(
                first_tag(&tracks, TagKey::OriginalReleaseDate)
                    .or_else(|| first_tag(&tracks, TagKey::OriginalReleaseYear)),
            ),
        }
    }
}
