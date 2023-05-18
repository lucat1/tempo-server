use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{sea_query::IntoValueTuple, Cursor, Order, SelectorTrait};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{cmp::Eq, collections::HashMap, default::Default, error::Error as StdError, hash::Hash};
use uuid::Uuid;

use super::documents::{
    ArtistAttributes, ArtistRelation, MediumAttributes, MediumRelation, ReleaseAttributes,
    ReleaseRelation, ServerAttributes, ServerRelation, TrackAttributes, TrackRelation,
};
use crate::documents::{
    ArtistCreditAttributes, ImageAttributes, ImageRelation, RecordingAttributes,
};

pub static DEFAULT_PAGE_SIZE: u32 = 10;
pub static MAX_PAGE_SIZE: u32 = 20;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Server,
    Image,
    Artist,
    Track,
    Medium,
    Release,
}

pub type ServerResource = Resource<String, ServerAttributes, ServerRelation>;
pub type ImageResource = Resource<String, ImageAttributes, ImageRelation>;
pub type ArtistResource = Resource<Uuid, ArtistAttributes, ArtistRelation>;
pub type TrackResource = Resource<Uuid, TrackAttributes, TrackRelation>;
pub type MediumResource = Resource<Uuid, MediumAttributes, MediumRelation>;
pub type ReleaseResource = Resource<Uuid, ReleaseAttributes, ReleaseRelation>;

#[derive(Serialize)]
#[serde(untagged)]
pub enum Included {
    Image(ImageResource),
    Artist(ArtistResource),
    Track(TrackResource),
    Medium(MediumResource),
    Release(ReleaseResource),
}

#[derive(Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LinkKey {
    Prev,
    Next,
    First,
    Last,
}

#[derive(Serialize)]
pub struct Document<R> {
    pub data: DocumentData<R>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub included: Vec<Included>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub links: HashMap<LinkKey, String>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum DocumentData<R> {
    Single(R),
    Multi(Vec<R>),
}

#[derive(Serialize)]
pub struct Resource<I, T, R> {
    pub r#type: ResourceType,
    pub id: I,
    pub attributes: T,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub relationships: HashMap<R, Relationship>,
}

#[derive(Serialize)]
pub struct Relationship {
    pub data: Relation,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Relation {
    Single(Related),
    Multi(Vec<Related>),
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Related {
    Artist(ResourceIdentifier<Uuid>),
    Track(ResourceIdentifier<Uuid>),
    Medium(ResourceIdentifier<Uuid>),
    Release(ResourceIdentifier<Uuid>),
    Image(ResourceIdentifier<String>),
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Meta {
    ArtistCredit(ArtistCreditAttributes),
    Recording(RecordingAttributes),
}

#[derive(Serialize)]
pub struct ResourceIdentifier<I> {
    pub r#type: ResourceType,
    pub id: I,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

pub struct Error {
    pub status: StatusCode,
    pub title: String,
    pub detail: Option<Box<dyn StdError>>,
}

#[derive(Serialize)]
pub struct SerializableError {
    pub status: u16,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let err = SerializableError {
            status: u16::from(self.status),
            title: self.title,
            detail: self.detail.map(|e| e.to_string()),
        };
        (self.status, Json(err)).into_response()
    }
}

fn default_page_size() -> u32 {
    DEFAULT_PAGE_SIZE
}

#[derive(Default, Debug, Deserialize, PartialEq, Eq, Hash, Validate)]
pub struct Page<Id: Default> {
    #[serde(default = "default_page_size")]
    #[validate(maximum = 20)]
    pub size: u32,
    pub before: Option<Id>,
    pub after: Option<Id>,
}

fn default_page<Id: Default>() -> Page<Id> {
    Page {
        size: default_page_size(),
        ..Default::default()
    }
}

#[derive(Deserialize)]
pub struct RawQueryOptions<Id: Default> {
    include: Option<String>,
    filter: Option<HashMap<String, String>>,
    sort: Option<String>,
    page: Option<Page<Id>>,
}

#[derive(Debug)]
pub struct QueryOptions<
    C: Eq + Hash + TryFrom<String>,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
> {
    pub include: Vec<I>,
    pub filter: HashMap<C, String>,
    pub sort: HashMap<C, Order>,
    pub page: Page<Id>,
}

pub struct Query<
    C: Eq + Hash + TryFrom<String>,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
>(pub QueryOptions<C, I, Id>);

#[async_trait]
impl<S, C, I, Id> FromRequestParts<S> for Query<C, I, Id>
where
    S: Send + Sync,
    C: Eq + Hash + TryFrom<String>,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query();
        match query {
            Some(qs) => {
                let raw_opts: RawQueryOptions<Id> = serde_qs::from_str(qs)
                    .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

                let parse_key = |k: &str| -> Option<C> { k.to_owned().try_into().ok() };

                let opts = QueryOptions {
                    include: raw_opts
                        .include
                        .as_ref()
                        .map(|s| -> Result<Vec<_>, serde_json::Error> {
                            s.split(',')
                                .map(|p| serde_json::from_str(&("\"".to_owned() + p + "\"")))
                                .collect::<Result<Vec<_>, serde_json::Error>>()
                        })
                        .unwrap_or(Ok(Vec::new()))
                        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?,
                    filter: raw_opts
                        .filter
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|(k, v)| -> Option<(C, String)> {
                            Some((parse_key(k.as_str())?, v))
                        })
                        .collect(),
                    sort: raw_opts
                        .sort
                        .map(|s| {
                            s.split(',')
                                .filter_map(|p| -> Option<(C, Order)> {
                                    if let Some(stripped_key) = s.strip_prefix('-') {
                                        Some((parse_key(stripped_key)?, Order::Desc))
                                    } else {
                                        Some((parse_key(p)?, Order::Asc))
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or(HashMap::new()),
                    page: raw_opts.page.unwrap_or_else(default_page),
                };
                Ok(Query(opts))
            }
            None => Ok(Self(QueryOptions {
                include: Vec::new(),
                filter: HashMap::new(),
                sort: HashMap::new(),
                page: Page {
                    size: DEFAULT_PAGE_SIZE,
                    ..Page::default()
                },
            })),
        }
    }
}

#[derive(PartialEq)]
enum Identifier {
    Image(String),
    Artist(Uuid),
    Track(Uuid),
    Medium(Uuid),
    Release(Uuid),
}

pub fn dedup(mut included: Vec<Included>) -> Vec<Included> {
    included.sort_unstable_by(|_a, _b| match (_a, _b) {
        (Included::Image(a), Included::Image(b)) => a.id.cmp(&b.id),
        (Included::Artist(a), Included::Artist(b)) => a.id.cmp(&b.id),
        (Included::Track(a), Included::Track(b)) => a.id.cmp(&b.id),
        (Included::Medium(a), Included::Medium(b)) => a.id.cmp(&b.id),
        (Included::Release(a), Included::Release(b)) => a.id.cmp(&b.id),
        (_, _) => std::cmp::Ordering::Less,
    });
    included.dedup_by_key(|e| match e {
        Included::Image(e) => Identifier::Image(e.id.to_owned()),
        Included::Artist(e) => Identifier::Artist(e.id),
        Included::Track(e) => Identifier::Track(e.id),
        Included::Medium(e) => Identifier::Medium(e.id),
        Included::Release(e) => Identifier::Release(e.id),
    });
    included
}

pub fn make_cursor<'a, S, Id>(mut cursor: &'a mut Cursor<S>, page: &Page<Id>) -> &'a mut Cursor<S>
where
    S: SelectorTrait,
    Id: Default + IntoValueTuple + Copy,
{
    if let Some(before) = page.before {
        cursor = cursor.before(before);
    }
    if let Some(after) = page.after {
        cursor = cursor.after(after);
    }
    cursor = match (page.after, page.before) {
        (None, Some(_)) => cursor.last(page.size.into()),
        // Also matches (Some(_), None), which means "everything all after `after`"
        (_, _) => cursor.first(page.size.into()),
    };
    cursor
}

pub fn links_from_resource<I, T, R, C, Inc, Id>(
    data: &[Resource<I, T, R>],
    _opts: &QueryOptions<C, Inc, Id>,
) -> HashMap<LinkKey, String>
where
    I: ToString,
    C: Eq + Hash + TryFrom<String>,
    Inc: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
{
    let mut res = HashMap::new();
    if let Some(first) = data.first() {
        res.insert(LinkKey::First, first.id.to_string());
    }
    if let Some(last) = data.last() {
        res.insert(LinkKey::Last, last.id.to_string());
    }

    // TODO: a good way to find out if there is a page before?
    // TODO: serialize next, previous from current url
    // if data.len() == page.size as usize {
    //     res.insert(LinkKey::Next);
    // }
    res
}
