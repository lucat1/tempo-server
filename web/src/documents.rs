use serde::{Deserialize, Serialize};
use uuid::Uuid;

use entity::RelationType;

#[derive(Serialize)]
pub struct ArtistCreditAttributes {
    pub join_phrase: Option<String>,
}

#[derive(Serialize)]
pub struct RecordingAttributes {
    pub role: RelationType,
    pub detail: String,
}

#[derive(Serialize)]
pub struct ImageAttributes {
    pub role: String,
    pub format: String,
    pub description: Option<String>,
    pub width: u32,
    pub height: u32,
    pub size: u32,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageRelation {
    #[serde(rename = "artists")]
    Artist,
    #[serde(rename = "releases")]
    Release,
    // TODO: tracks?
}

#[derive(Serialize)]
pub struct ArtistAttributes {
    pub name: String,
    pub sort_name: String,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtistRelation {
    #[serde(rename = "recordings")]
    Recordings,
    #[serde(rename = "images")]
    Images,
    #[serde(rename = "releases")]
    Releases,
    #[serde(rename = "tracks")]
    Tracks,
}

#[derive(Deserialize, PartialEq, Eq)]
pub enum ArtistInclude {
    #[serde(rename = "images")]
    Images,
    #[serde(rename = "releases")]
    Releases,
    #[serde(rename = "tracks")]
    Tracks,
}

#[derive(Serialize)]
pub struct ReleaseAttributes {
    pub title: String,
    pub disctotal: u32,
    pub tracktotal: u32,
    pub genres: Vec<String>,

    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub original_year: Option<i32>,
    pub original_month: Option<u32>,
    pub original_day: Option<u32>,

    #[serde(rename = "release-type")]
    pub release_type: Option<String>,
    #[serde(rename = "release-mbid")]
    pub release_mbid: Uuid,
    #[serde(rename = "release-group-mbid")]
    pub release_group_mbid: Option<Uuid>,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseRelation {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "mediums")]
    Mediums,
    #[serde(rename = "artists")]
    Artists,
}

#[derive(Deserialize, PartialEq, Eq)]
pub enum ReleaseInclude {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "mediums")]
    Mediums,
    #[serde(rename = "artists")]
    Artists,
}

#[derive(Serialize)]
pub struct MediumAttributes {
    pub position: u32,
    pub tracks: u32,
    #[serde(rename = "track-offset")]
    pub track_offset: u32,
    pub format: Option<String>,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediumRelation {
    Release,
    Tracks,
}

#[derive(Deserialize, PartialEq, Eq)]
pub enum MediumInclude {
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "tracks")]
    Tracks,
}

#[derive(Serialize)]
pub struct TrackAttributes {
    pub title: String,
    pub track: u32,
    pub disc: u32,
    pub genres: Vec<String>,
    pub bpm: Option<u32>,

    #[serde(rename = "recording-mbid")]
    pub recording_mbid: Uuid,
    #[serde(rename = "track-mbid")]
    pub track_mbid: Uuid,
    pub comments: Option<String>,

    pub mimetype: String,
    pub duration: Option<f32>,
    pub framerate: Option<u32>,
    pub framecount: Option<u32>,
    pub channels: Option<u32>,
    pub bitrate: Option<u32>,
    pub bitdepth: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackRelation {
    Artists,
    Medium,
    Recorders,
}
