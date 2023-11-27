use sea_orm::ColumnTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::api::jsonapi::{InsertResource, Resource};
use entity::{ArtistTrackRelationType, ArtistUrlType, ConnectionProvider};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Server,
    Auth,
    User,
    Scrobble,
    Connection,

    Image,
    Artist,
    Track,
    Medium,
    Release,
    Genre,
}

pub type ServerResource = Resource<ResourceType, String, ServerAttributes, ServerRelation, Meta>;
pub type AuthResource = Resource<ResourceType, String, AuthAttributes, AuthRelation, Meta>;
pub type UserResource = Resource<ResourceType, String, UserAttributes, UserRelation, Meta>;
pub type ScrobbleResource = Resource<ResourceType, i64, ScrobbleAttributes, ScrobbleRelation, Meta>;
pub type ConnectionResource =
    Resource<ResourceType, ConnectionProvider, ConnectionAttributes, ConnectionRelation, Meta>;
pub type ImageResource = Resource<ResourceType, String, ImageAttributes, ImageRelation, Meta>;
pub type ArtistResource = Resource<ResourceType, Uuid, ArtistAttributes, ArtistRelation, Meta>;
pub type TrackResource = Resource<ResourceType, Uuid, TrackAttributes, TrackRelation, Meta>;
pub type MediumResource = Resource<ResourceType, Uuid, MediumAttributes, MediumRelation, Meta>;
pub type ReleaseResource = Resource<ResourceType, Uuid, ReleaseAttributes, ReleaseRelation, Meta>;
pub type GenreResource = Resource<ResourceType, String, GenreAttributes, GenreRelation, Meta>;

// pub type InsertServerResource = InsertResource<ServerAttributes, ServerRelation>;
// pub type InsertAuthResource = InsertResource<AuthAttributes, AuthRelation>;
// pub type InsertUserResource = InsertResource<UserAttributes, UserRelation>;
pub type InsertScrobbleResource =
    InsertResource<ResourceType, ScrobbleAttributes, ScrobbleRelation, Meta>;
// pub type InsertImageResource = InsertResource<ImageAttributes, ImageRelation>;
// pub type InsertArtistResource = InsertResource<ArtistAttributes, ArtistRelation>;
// pub type InsertTrackResource = InsertResource<TrackAttributes, TrackRelation>;
// pub type InsertMediumResource = InsertResource<MediumAttributes, MediumRelation>;
// pub type InsertReleaseResource = InsertResource<ReleaseAttributes, ReleaseRelation>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Included {
    User(UserResource),
    Scrobble(ScrobbleResource),
    Image(ImageResource),
    Artist(ArtistResource),
    Track(TrackResource),
    Medium(MediumResource),
    Release(ReleaseResource),
    Genre(GenreResource),
}

impl PartialEq for Included {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Included::Image(a), Included::Image(b)) => a.id == b.id,
            (Included::Artist(a), Included::Artist(b)) => a.id == b.id,
            (Included::Track(a), Included::Track(b)) => a.id == b.id,
            (Included::Medium(a), Included::Medium(b)) => a.id == b.id,
            (Included::Release(a), Included::Release(b)) => a.id == b.id,
            (_, _) => false,
        }
    }
}
impl Eq for Included {}

impl std::cmp::PartialOrd for Included {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Included::Image(a), Included::Image(b)) => a.id.partial_cmp(&b.id),
            (Included::Artist(a), Included::Artist(b)) => a.id.partial_cmp(&b.id),
            (Included::Track(a), Included::Track(b)) => a.id.partial_cmp(&b.id),
            (Included::Medium(a), Included::Medium(b)) => a.id.partial_cmp(&b.id),
            (Included::Release(a), Included::Release(b)) => a.id.partial_cmp(&b.id),
            (_, _) => None,
        }
    }
}

impl std::cmp::Ord for Included {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Included::Image(a), Included::Image(b)) => a.id.cmp(&b.id),
            (Included::Artist(a), Included::Artist(b)) => a.id.cmp(&b.id),
            (Included::Track(a), Included::Track(b)) => a.id.cmp(&b.id),
            (Included::Medium(a), Included::Medium(b)) => a.id.cmp(&b.id),
            (Included::Release(a), Included::Release(b)) => a.id.cmp(&b.id),
            (_, _) => std::cmp::Ordering::Less,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Meta {
    ArtistCredit(ArtistCreditAttributes),
    Genre(GenreMetaAttributes),
    Recording(RecordingAttributes),
    Connection(ConnectionMetaAttributes),

    SearchResult(SearchResultAttributes),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchResultAttributes {
    pub score: f32,
}

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
#[serde(rename_all = "snake_case")]
pub enum ServerRelation {}

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
    Artist,
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
    Recordings,
    Images,
    Releases,
    Tracks,
    Picture,
    Cover,
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
    Image,
    Mediums,
    Artists,
    Genres,
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
    #[serde(rename = "genres")]
    Genres,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<i32>,

    pub recording_mbid: Uuid,
    pub track_mbid: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,

    pub mimetype: Option<String>,
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
    Genres,
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
    #[serde(rename = "genres")]
    Genres,
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

#[derive(Serialize, Deserialize)]
pub struct GenreAttributes {
    pub name: String,
    pub disambiguation: String,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenreRelation {
    Tracks,
    Releases,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GenreInclude {
    #[serde(rename = "tracks")]
    Tracks,
    #[serde(rename = "releases")]
    Releases,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum GenreFilter {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "disambiguation")]
    Disambiguation,

    Include(GenreInclude),
}

impl IntoColumn<entity::GenreColumn> for GenreFilter {
    fn column(&self) -> Option<entity::GenreColumn> {
        match self {
            GenreFilter::Name => Some(entity::GenreColumn::Name),
            GenreFilter::Disambiguation => Some(entity::GenreColumn::Disambiguation),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenreMetaAttributes {
    pub count: i32,
}
