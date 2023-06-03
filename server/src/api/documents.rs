use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

use entity::{ArtistTrackRelationType, ArtistUrlType};

#[derive(Serialize, Deserialize)]
pub struct ServerAttributes {
    pub tempo_version: String,
    pub server: String,
    pub server_version: String,
    pub auth_required: bool,
    pub features: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServerRelation {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArtistCreditAttributes {
    pub join_phrase: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordingAttributes {
    pub role: ArtistTrackRelationType,
    pub detail: String,
}

#[derive(Serialize, Deserialize)]
pub struct ImageAttributes {
    pub role: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    pub size: i32,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageRelation {
    #[serde(rename = "artists")]
    Artist,
    #[serde(rename = "releases")]
    Release,
    // TODO: tracks?
}

#[derive(Serialize, Deserialize)]
pub struct ArtistAttributes {
    pub name: String,
    pub sort_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub urls: HashMap<ArtistUrlType, String>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize)]
pub struct ReleaseAttributes {
    pub title: String,
    pub disctotal: i32,
    pub tracktotal: i32,
    pub genres: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_month: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_day: Option<i16>,

    #[serde(rename = "release_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_type: Option<String>,
    #[serde(rename = "release_mbid")]
    pub release_mbid: Uuid,
    #[serde(rename = "release_group_mbid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_group_mbid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseRelation {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "mediums")]
    Mediums,
    #[serde(rename = "artists")]
    Artists,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize)]
pub struct MediumAttributes {
    pub position: i32,
    pub tracks: i32,
    pub track_offset: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediumRelation {
    Release,
    Tracks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediumInclude {
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "release.artists")]
    ReleaseArtists,
    #[serde(rename = "tracks")]
    Tracks,
    #[serde(rename = "tracks.artists")]
    TracksArtists,
}

#[derive(Serialize, Deserialize)]
pub struct TrackAttributes {
    pub title: String,
    pub track: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disc: Option<i32>,
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<i32>,

    pub recording_mbid: Uuid,
    pub track_mbid: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,

    pub mimetype: String,
    pub duration: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framerate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framecount: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitdepth: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i32>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackRelation {
    Artists,
    Medium,
    Recorders,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrackInclude {
    #[serde(rename = "artists")]
    Artists,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "medium.release")]
    MediumRelease,
    #[serde(rename = "medium.release.artists")]
    MediumReleaseArtists,
    #[serde(rename = "recorders")]
    Recorders,
}

#[derive(Serialize, Deserialize)]
pub struct AuthAttributes {
    pub token: Token,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<Token>,
}

#[derive(Serialize, Deserialize)]
pub struct Token {
    pub value: String,
    #[serde(with = "time::serde::iso8601")]
    pub expires_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthRelation {
    User,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserAttributes {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserRelation {
    Scrobbles,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserInclude {
    #[serde(rename = "scrobbles")]
    Scrobbles,
    #[serde(rename = "scrobbles.tracks")]
    ScrobblesTracks,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrobbleAttributes {
    #[serde(with = "time::serde::iso8601")]
    pub at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScrobbleRelation {
    User,
    Track,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScrobbleInclude {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "track")]
    Track,
    #[serde(rename = "track.artists")]
    TrackArtists,
    #[serde(rename = "track.medium")]
    TrackMedium,
    #[serde(rename = "track.medium.release")]
    TrackMediumRelease,
    #[serde(rename = "track.medium.release.artists")]
    TrackMediumReleaseArtists,
}
