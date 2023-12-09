use const_format::formatcp;
use eyre::Result;
use governor::{clock::*, middleware::*, state::*, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero_ext::*;
use reqwest::{header::HeaderValue, header::USER_AGENT, Error, Request, Response};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use uuid::Uuid;

pub static MB_BASE_STRURL: &str = "https://musicbrainz.org/ws/2/";
static MB_CALLS_PER_SECOND: NonZeroU32 = nonzero!(1u32);

lazy_static! {
    pub static ref MB_BASE_URL: url::Url = url::Url::parse(MB_BASE_STRURL).unwrap();
    static ref UNLIMITED_CLIENT: reqwest::Client = reqwest::Client::new();
    static ref LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware> =
        RateLimiter::direct(Quota::per_second(MB_CALLS_PER_SECOND));
    static ref MB_USER_AGENT: HeaderValue =
        formatcp!("{}/{} ({})", base::CLI_NAME, base::VERSION, base::GITHUB)
            .parse()
            .unwrap();
}

pub async fn send_request(mut req: Request) -> Result<Response, Error> {
    LIMITER.until_ready().await;
    let headers = req.headers_mut();
    headers.append(USER_AGENT, MB_USER_AGENT.clone());
    UNLIMITED_CLIENT.execute(req).await
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    #[serde(rename = "label-info")]
    #[serde(default)]
    pub label_info: Vec<LabelInfo>,
    pub status: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<ReleaseGroup>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    pub asin: Option<String>,
    pub date: Option<String>,
    pub id: Uuid,
    pub media: Option<Vec<Medium>>,
    pub country: Option<String>,
    #[serde(rename = "text-representation")]
    pub text_representation: Option<TextRepresentation>,
    pub title: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "track-count")]
    pub track_count: Option<usize>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGroup {
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: String,
    pub id: Uuid,
    pub disambiguation: Option<String>,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub joinphrase: Option<String>,
    pub artist: Artist,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    #[serde(rename = "type-id")]
    pub type_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub disambiguation: Option<String>,
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Vec<String>,
    pub id: Uuid,
    pub disambiguation: Option<String>,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Medium {
    pub id: Option<Uuid>,
    pub position: Option<i32>,
    #[serde(rename = "track-offset")]
    pub track_offset: Option<i32>,
    #[serde(rename = "track-count")]
    pub track_count: i32,
    pub tracks: Option<Vec<Track>>,
    pub format: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: Uuid,
    pub recording: Recording,
    pub number: String,
    pub position: i32,
    pub length: Option<i32>,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub relations: Vec<Relation>,
    pub disambiguation: String,
    pub id: Uuid,
    pub length: Option<i32>,
    pub video: bool,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: Option<String>,
    pub genres: Option<Vec<Genre>>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "type")]
    pub type_field: String,
    pub artist: Option<Artist>,
    pub work: Option<Work>,
    pub attributes: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    pub relations: Option<Vec<Relation>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genre {
    pub id: Uuid,
    pub count: u32,
    pub disambiguation: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSearch {
    pub created: String,
    pub count: i64,
    pub offset: i64,
    pub releases: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRepresentation {
    pub language: Option<String>,
    pub script: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
    pub label: Option<Label>,
}
