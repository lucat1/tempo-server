use crate::fetch::{ArtistLike, ReleaseLike};
use crate::track::TrackLike;
use eyre::Result;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use super::MusicBrainz;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub disambiguation: Option<String>,
    #[serde(rename = "label-info")]
    #[serde(default)]
    pub label_info: Vec<LabelInfo>,
    pub status: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<ReleaseGroup>,
    #[serde(rename = "cover-art-archive")]
    pub cover_art_archive: Option<CoverArtArchive>,
    #[serde(rename = "status-id")]
    pub status_id: Option<String>,
    pub packaging: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    pub asin: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "release-events")]
    #[serde(default)]
    pub release_events: Vec<Event>,
    pub id: String,
    pub barcode: Option<String>,
    pub quality: Option<String>,
    pub media: Vec<Medium>,
    pub country: Option<String>,
    #[serde(rename = "packaging-id")]
    pub packaging_id: Option<String>,
    #[serde(rename = "text-representation")]
    pub text_representation: Option<TextRepresentation>,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(rename = "track-count")]
    pub track_count: Option<usize>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
    pub name: String,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGroup {
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: String,
    #[serde(rename = "primary-type-id")]
    pub primary_type_id: String,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "primary-type")]
    pub primary_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverArtArchive {
    pub count: i64,
    pub front: bool,
    pub back: bool,
    pub artwork: bool,
    pub darkened: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub joinphrase: Option<String>,
    pub artist: Artist,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub disambiguation: Option<String>,
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub area: Area,
    pub date: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Area {
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Vec<String>,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Medium {
    #[serde(rename = "disc-count")]
    pub disc_count: Option<i64>,
    #[serde(rename = "track-count")]
    pub track_count: i64,
    pub format: String,
    pub title: Option<String>,
    #[serde(rename = "format-id")]
    pub format_id: Option<String>,
    #[serde(rename = "track-offset")]
    pub track_offset: Option<i64>,
    pub tracks: Option<Vec<Track>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub recording: Recording,
    pub number: String,
    pub position: i64,
    pub length: u64,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recording {
    pub disambiguation: String,
    pub id: String,
    pub length: i64,
    pub video: bool,
    #[serde(rename = "first-release-date")]
    pub first_release_date: String,
    pub title: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseSearch {
    pub created: String,
    pub count: i64,
    pub offset: i64,
    pub releases: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRepresentation {
    pub language: Option<String>,
    pub script: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
    pub label: Option<Label>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub count: i64,
    pub name: String,
}

impl ArtistLike for ArtistCredit {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn mbid(&self) -> Option<String> {
        Some(self.artist.id.clone())
    }
    fn joinphrase(&self) -> Option<String> {
        self.joinphrase.clone()
    }
}

impl TrackLike for Track {
    fn title(&self) -> String {
        self.title.clone()
    }
    fn length(&self) -> u64 {
        self.length
    }
}

impl ReleaseLike for Release {
    fn fetcher(&self) -> Option<Box<dyn crate::fetch::Fetch>> {
        // TODO: should be recevied from its "parent"
        Some(Box::new(MusicBrainz::new(None, None)))
    }
    fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }

    fn artists(&self) -> Vec<Box<dyn ArtistLike>> {
        self.artist_credit
            .iter()
            .map(|a| Box::new(a.clone()) as Box<dyn ArtistLike>)
            .collect::<Vec<_>>()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn tracks(&self) -> Option<Vec<Box<dyn TrackLike>>> {
        let tracks = self
            .media
            .iter()
            .filter_map(|media| media.tracks.clone())
            .flatten()
            .collect::<Vec<_>>();
        if tracks.len() <= 0 {
            None
        } else {
            Some(
                tracks
                    .iter()
                    .map(|t| Box::new(t.clone()) as Box<dyn TrackLike>)
                    .collect(),
            )
        }
    }
}
