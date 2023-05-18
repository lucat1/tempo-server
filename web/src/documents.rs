use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use entity::{ArtistTrackRelationType, ArtistUrlType};

#[derive(Serialize)]
pub struct ServerAttributes {
    #[serde(rename = "aura-version")]
    pub aura_version: String,
    pub server: String,
    #[serde(rename = "server-version")]
    pub server_version: String,
    #[serde(rename = "auth-required")]
    pub auth_required: bool,
    pub features: Vec<String>,
}

#[derive(Serialize)]
pub struct ServerRelation {}

#[derive(Serialize)]
pub struct ArtistCreditAttributes {
    pub join_phrase: Option<String>,
}

#[derive(Serialize)]
pub struct RecordingAttributes {
    pub role: ArtistTrackRelationType,
    pub detail: String,
}

#[derive(Serialize)]
pub struct ImageAttributes {
    pub role: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub urls: HashMap<ArtistUrlType, String>,
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

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum ArtistInclude {
    #[serde(rename = "images")]
    Images,
    #[serde(rename = "tracks")]
    Tracks,
    #[serde(rename = "releases")]
    Releases,
    #[serde(rename = "releases.artists")]
    ReleasesArtists,
}

#[derive(Serialize)]
pub struct ReleaseAttributes {
    pub title: String,
    pub disctotal: u32,
    pub tracktotal: u32,
    pub genres: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_month: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_day: Option<u32>,

    #[serde(rename = "release-type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_type: Option<String>,
    #[serde(rename = "release-mbid")]
    pub release_mbid: Uuid,
    #[serde(rename = "release-group-mbid")]
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum ReleaseInclude {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "artists")]
    Artists,
    #[serde(rename = "mediums")]
    Mediums,
    #[serde(rename = "mediums.tracks")]
    MediumsTracks,
    #[serde(rename = "mediums.tracks.artists")]
    MediumsTracksArtists,
}

#[derive(Serialize)]
pub struct MediumAttributes {
    pub position: u32,
    pub tracks: u32,
    #[serde(rename = "track-offset")]
    pub track_offset: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediumRelation {
    Release,
    Tracks,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum MediumInclude {
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "tracks")]
    Tracks,
    #[serde(rename = "tracks.artists")]
    TracksArtists,
}

#[derive(Serialize)]
pub struct TrackAttributes {
    pub title: String,
    pub track: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disc: Option<u32>,
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<u32>,

    #[serde(rename = "recording-mbid")]
    pub recording_mbid: Uuid,
    #[serde(rename = "track-mbid")]
    pub track_mbid: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,

    pub mimetype: String,
    pub duration: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framerate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framecount: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitdepth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackRelation {
    Artists,
    Medium,
    Recorders,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum TrackInclude {
    #[serde(rename = "artists")]
    Artists,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "recorders")]
    Recorders,
}
