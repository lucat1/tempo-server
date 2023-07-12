use entity::{
    full::{FullRelease, FullTrack},
    InternalRelease, InternalTrack,
};
use itertools::Itertools;
use serde::Serialize;
use tag::TagKey;

use crate::track::TrackFile;
use base::util::{dedup, maybe_date};

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";
pub const UNKNOWN_TITLE: &str = "(unkown title)";

pub trait IntoInternal<T> {
    fn into_internal(self) -> T;
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
        tracing::warn! {
            n_of_tags = options.len(),
            %tag,
            values = options.join(", "),
            "Multiple unique tag values for the required tag in the given tracks",
        };
    }
    options.first().map(|f| f.to_string())
}

fn artists_with_name(name: String, sep: Option<String>) -> Vec<String> {
    match sep {
        Some(ref s) => name.split(s.as_str()).map(|s| s.to_string()).collect_vec(),
        None => vec![name],
    }
}

impl IntoInternal<InternalRelease> for Vec<TrackFile> {
    fn into_internal(self) -> InternalRelease {
        let artists = if first_tag(&self, TagKey::AlbumArtist).is_some() {
            // Use the AlbumArtist to search if we have one available
            artists_from_tag(&self, TagKey::AlbumArtist)
        } else {
            // Otherwise use the Artist tag
            let mut v1 = artists_from_tag(&self, TagKey::Artist);
            let mut v2 = artists_from_tag(&self, TagKey::Artists);
            v1.append(&mut v2);
            v1
        };

        let date = maybe_date(
            first_tag(&self, TagKey::ReleaseDate).or_else(|| first_tag(&self, TagKey::ReleaseYear)),
        );
        let original_date = maybe_date(
            first_tag(&self, TagKey::OriginalReleaseDate)
                .or_else(|| first_tag(&self, TagKey::OriginalReleaseYear)),
        );
        InternalRelease {
            title: first_tag(&self, TagKey::Album).unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists,
            discs: first_tag(&self, TagKey::TotalDiscs).and_then(|d| d.parse::<i32>().ok()),
            media: first_tag(&self, TagKey::Media),
            tracks: first_tag(&self, TagKey::TotalTracks).and_then(|d| d.parse::<i32>().ok()),
            country: first_tag(&self, TagKey::ReleaseCountry),
            label: first_tag(&self, TagKey::RecordLabel),
            release_type: first_tag(&self, TagKey::ReleaseType),
            year: date.year,
            month: date.month,
            day: date.day,
            original_year: original_date.year,
            original_month: original_date.month,
            original_day: original_date.day,
        }
    }
}

impl IntoInternal<InternalTrack> for TrackFile {
    fn into_internal(self) -> InternalTrack {
        let file_singleton = vec![self];
        InternalTrack {
            title: first_tag(&file_singleton, TagKey::TrackTitle)
                .unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists: artists_from_tag(&file_singleton, TagKey::Artists),
            length: first_tag(&file_singleton, TagKey::Duration)
                .and_then(|d| d.parse::<i32>().ok()),
            disc: first_tag(&file_singleton, TagKey::DiscNumber)
                .and_then(|d| d.parse::<i32>().ok()),
            number: first_tag(&file_singleton, TagKey::TrackNumber)
                .and_then(|d| d.parse::<i32>().ok()),
        }
    }
}
