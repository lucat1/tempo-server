use log::warn;
use tag::TagKey;

use crate::internal::UNKNOWN_TITLE;
use crate::internal::{Release, Track};
use crate::track::TrackFile;
use crate::util::{dedup, maybe_date};

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
                .and_then(|d| d.parse::<u64>().ok()),
            disc: first_tag(&file_singleton, TagKey::DiscNumber)
                .and_then(|d| d.parse::<u64>().ok()),
            number: first_tag(&file_singleton, TagKey::TrackNumber)
                .and_then(|d| d.parse::<u64>().ok()),
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
            discs: first_tag(&tracks, TagKey::TotalDiscs).and_then(|d| d.parse::<u64>().ok()),
            media: first_tag(&tracks, TagKey::Media),
            tracks: first_tag(&tracks, TagKey::TotalTracks).and_then(|d| d.parse::<u64>().ok()),
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
