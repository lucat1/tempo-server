use sea_orm::ColumnTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use entity::{ArtistTrackRelationType, ArtistUrlType};

pub trait IntoColumn<T>
where
    T: ColumnTrait,
{
    fn column(&self) -> Option<T>;
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtistCreditAttributes {
    pub join_phrase: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ArtistFilter {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "sort_name")]
    SortName,
    #[serde(rename = "description")]
    Description,

    Include(ArtistInclude),
}

impl IntoColumn<entity::ArtistColumn> for ArtistFilter {
    fn column(&self) -> Option<entity::ArtistColumn> {
        match self {
            ArtistFilter::Id => Some(entity::ArtistColumn::Id),
            ArtistFilter::Name => Some(entity::ArtistColumn::Name),
            ArtistFilter::SortName => Some(entity::ArtistColumn::SortName),
            ArtistFilter::Description => Some(entity::ArtistColumn::Description),
            _ => None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ReleaseFilter {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "Disctotal")]
    Disctotal,
    #[serde(rename = "tracktotal")]
    Tracktotal,
    #[serde(rename = "genres")]
    Genres,
    #[serde(rename = "year")]
    Year,
    #[serde(rename = "month")]
    Month,
    #[serde(rename = "day")]
    Day,
    #[serde(rename = "original_year")]
    OriginalYear,
    #[serde(rename = "original_month")]
    OriginalMonth,
    #[serde(rename = "original_day")]
    OriginalDay,
    #[serde(rename = "release_type")]
    ReleaseType,
    #[serde(rename = "release_mbid")]
    ReleaseMbid,
    #[serde(rename = "release_group_mbid")]
    ReleaseGroupMbid,

    Include(ReleaseInclude),
}

impl IntoColumn<entity::ReleaseColumn> for ReleaseFilter {
    fn column(&self) -> Option<entity::ReleaseColumn> {
        match self {
            ReleaseFilter::Id => Some(entity::ReleaseColumn::Id),
            ReleaseFilter::Title => Some(entity::ReleaseColumn::Title),
            ReleaseFilter::Genres => Some(entity::ReleaseColumn::Genres),
            ReleaseFilter::Year => Some(entity::ReleaseColumn::Year),
            ReleaseFilter::Month => Some(entity::ReleaseColumn::Month),
            ReleaseFilter::Day => Some(entity::ReleaseColumn::Day),
            ReleaseFilter::OriginalYear => Some(entity::ReleaseColumn::OriginalYear),
            ReleaseFilter::OriginalMonth => Some(entity::ReleaseColumn::OriginalMonth),
            ReleaseFilter::OriginalDay => Some(entity::ReleaseColumn::OriginalDay),
            ReleaseFilter::ReleaseType => Some(entity::ReleaseColumn::ReleaseType),
            ReleaseFilter::ReleaseMbid => Some(entity::ReleaseColumn::Id),
            ReleaseFilter::ReleaseGroupMbid => Some(entity::ReleaseColumn::ReleaseGroupId),
            _ => None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum MediumFilter {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "position")]
    Position,
    #[serde(rename = "tracks")]
    Tracks,
    #[serde(rename = "track_offset")]
    TrackOffset,
    #[serde(rename = "format")]
    Format,

    Include(MediumInclude),
}

impl IntoColumn<entity::MediumColumn> for MediumFilter {
    fn column(&self) -> Option<entity::MediumColumn> {
        match self {
            MediumFilter::Id => Some(entity::MediumColumn::Id),
            MediumFilter::Position => Some(entity::MediumColumn::Position),
            MediumFilter::Tracks => Some(entity::MediumColumn::Tracks),
            MediumFilter::TrackOffset => Some(entity::MediumColumn::TrackOffset),
            MediumFilter::Format => Some(entity::MediumColumn::Format),
            _ => None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum TrackFilter {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "track")]
    Track,
    #[serde(rename = "disc")]
    Disc,
    #[serde(rename = "genres")]
    Genres,
    #[serde(rename = "bpm")]
    Bpm,

    #[serde(rename = "recording_mbid")]
    RecordingMbid,
    #[serde(rename = "track_mbid")]
    TrackMbid,
    #[serde(rename = "comments")]
    Comments,

    #[serde(rename = "mimetype")]
    Mimetype,
    #[serde(rename = "duration")]
    Duration,
    #[serde(rename = "framerate")]
    Framerate,
    #[serde(rename = "framecount")]
    Framecount,
    #[serde(rename = "channels")]
    Channels,
    #[serde(rename = "birate")]
    Bitrate,
    #[serde(rename = "bitdepth")]
    Bitdepth,
    #[serde(rename = "size")]
    Size,

    Include(TrackInclude),
}

impl IntoColumn<entity::TrackColumn> for TrackFilter {
    fn column(&self) -> Option<entity::TrackColumn> {
        match self {
            TrackFilter::Id => Some(entity::TrackColumn::Id),
            TrackFilter::Title => Some(entity::TrackColumn::Title),
            TrackFilter::Track => Some(entity::TrackColumn::Number),
            TrackFilter::Genres => Some(entity::TrackColumn::Genres),
            TrackFilter::RecordingMbid => Some(entity::TrackColumn::RecordingId),
            TrackFilter::TrackMbid => Some(entity::TrackColumn::Id),
            TrackFilter::Mimetype => Some(entity::TrackColumn::Format),
            TrackFilter::Duration => Some(entity::TrackColumn::Length),
            _ => None,
        }
    }
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
    Connections,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UserInclude {
    #[serde(rename = "scrobbles")]
    Scrobbles,
    #[serde(rename = "scrobbles.track")]
    ScrobblesTracks,
    #[serde(rename = "scrobbles.track.artists")]
    ScrobblesTracksArtists,
    #[serde(rename = "scrobbles.track.medium")]
    ScrobblesTracksMedium,
    #[serde(rename = "scrobbles.track.medium.release")]
    ScrobblesTracksMediumRelease,
    #[serde(rename = "scrobbles.track.medium.release.artists")]
    ScrobblesTracksMediumReleaseArtists,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum UserFilter {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "first_name")]
    FirstName,
    #[serde(rename = "last_name")]
    LastName,

    Include(UserInclude),
}

impl IntoColumn<entity::UserColumn> for UserFilter {
    fn column(&self) -> Option<entity::UserColumn> {
        match self {
            UserFilter::Id => Some(entity::UserColumn::Username),
            UserFilter::FirstName => Some(entity::UserColumn::FirstName),
            UserFilter::LastName => Some(entity::UserColumn::LastName),
            _ => None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ScrobbleFilter {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "at")]
    At,

    Include(ScrobbleInclude),
}

impl IntoColumn<entity::ScrobbleColumn> for ScrobbleFilter {
    fn column(&self) -> Option<entity::ScrobbleColumn> {
        match self {
            ScrobbleFilter::Id => Some(entity::ScrobbleColumn::Id),
            ScrobbleFilter::At => Some(entity::ScrobbleColumn::At),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionFlow {
    Redirect,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionAttributes {
    pub homepage: Url,
    pub flow: ConnectionFlow,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionRelation {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionMetaAttributes {
    pub username: String,
    pub profile_url: Url,
}
