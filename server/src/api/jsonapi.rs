use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use itertools::Itertools;
use sea_orm::{
    sea_query::{IntoIden, IntoValueTuple},
    ColumnTrait, Cursor, Order, SelectorTrait,
};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{cmp::Eq, collections::HashMap, default::Default, error::Error as StdError, hash::Hash};
use uuid::Uuid;

use super::documents::{
    ArtistAttributes, ArtistCreditAttributes, ArtistRelation, AuthAttributes, AuthRelation,
    ImageAttributes, ImageRelation, MediumAttributes, MediumRelation, RecordingAttributes,
    ReleaseAttributes, ReleaseRelation, ScrobbleAttributes, ScrobbleRelation, ServerAttributes,
    ServerRelation, TrackAttributes, TrackRelation,
};

pub static DEFAULT_PAGE_SIZE: u32 = 10;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Server,
    Auth,
    User,
    Scrobble,

    Image,
    Artist,
    Track,
    Medium,
    Release,
}

pub type ServerResource = Resource<String, ServerAttributes, ServerRelation>;
pub type AuthResource = Resource<String, AuthAttributes, AuthRelation>;
pub type ScrobbleResource = Resource<i64, ScrobbleAttributes, ScrobbleRelation>;

pub type ImageResource = Resource<String, ImageAttributes, ImageRelation>;
pub type ArtistResource = Resource<Uuid, ArtistAttributes, ArtistRelation>;
pub type TrackResource = Resource<Uuid, TrackAttributes, TrackRelation>;
pub type MediumResource = Resource<Uuid, MediumAttributes, MediumRelation>;
pub type ReleaseResource = Resource<Uuid, ReleaseAttributes, ReleaseRelation>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Included {
    Image(ImageResource),
    Artist(ArtistResource),
    Track(TrackResource),
    Medium(MediumResource),
    Release(ReleaseResource),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LinkKey {
    Prev,
    Next,
    First,
    Last,
}

#[derive(Serialize, Deserialize)]
pub struct Document<R> {
    pub data: DocumentData<R>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub included: Vec<Included>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub links: HashMap<LinkKey, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocumentData<R> {
    Single(R),
    Multi(Vec<R>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMetaKey {
    Score,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resource<I, T, R: Eq + Hash> {
    pub r#type: ResourceType,
    pub id: I,
    pub attributes: T,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    pub relationships: HashMap<R, Relationship>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    pub meta: HashMap<ResourceMetaKey, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Relationship {
    pub data: Relation,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Relation {
    Single(Related),
    Multi(Vec<Related>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Related {
    User(ResourceIdentifier<String>),

    Artist(ResourceIdentifier<Uuid>),
    Track(ResourceIdentifier<Uuid>),
    Medium(ResourceIdentifier<Uuid>),
    Release(ResourceIdentifier<Uuid>),
    Image(ResourceIdentifier<String>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Meta {
    ArtistCredit(ArtistCreditAttributes),
    Recording(RecordingAttributes),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResourceIdentifier<I> {
    pub r#type: ResourceType,
    pub id: I,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub meta: Option<Meta>,
}

pub struct Error {
    pub status: StatusCode,
    pub title: String,
    pub detail: Option<Box<dyn StdError>>,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash, Validate)]
pub struct Page<Id: Default> {
    #[serde(default = "default_page_size")]
    #[validate(maximum = 20)] // max page size
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

#[derive(Deserialize, Serialize)]
pub struct RawQueryOptions<Id: Default> {
    include: Option<String>,
    filter: Option<HashMap<String, String>>,
    sort: Option<String>,
    page: Option<Page<Id>>,
}

#[derive(Debug, Clone)]
pub struct QueryOptions<
    C: Eq + Hash,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
> {
    pub include: Vec<I>,
    pub filter: HashMap<C, String>,
    pub sort: HashMap<C, Order>,
    pub page: Page<Id>,
}

pub struct Query<C: Eq + Hash, I: for<'a> Deserialize<'a>, Id: for<'a> Deserialize<'a> + Default>(
    pub QueryOptions<C, I, Id>,
);

#[async_trait]
impl<S, C, I, Id> FromRequestParts<S> for Query<C, I, Id>
where
    S: Send + Sync,
    C: Eq + Hash + ColumnTrait,
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

                let parse_key = |k: &str| -> Option<C> { C::from_str(k).ok() };

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

impl<C, I, Id> From<QueryOptions<C, I, Id>> for RawQueryOptions<Id>
where
    C: Eq + Hash + ColumnTrait,
    I: for<'a> Deserialize<'a> + Serialize,
    Id: for<'a> Deserialize<'a> + Default,
{
    fn from(value: QueryOptions<C, I, Id>) -> Self {
        RawQueryOptions {
            include: if !value.include.is_empty() {
                Some(
                    value
                        .include
                        .iter()
                        .filter_map(|i| {
                            serde_json::to_string(i)
                                .ok()
                                .map(|s| s[1..s.len() - 1].to_string())
                        })
                        .join(","),
                )
            } else {
                None
            },
            filter: Some(
                value
                    .filter
                    .into_iter()
                    .map(|(k, v)| (k.into_iden().to_string(), v))
                    .collect(),
            ),
            sort: Some(
                value
                    .sort
                    .into_iter()
                    .map(|(k, v)| {
                        match v {
                            Order::Asc => "",
                            Order::Desc => "-",
                            Order::Field(_) => unreachable!(),
                        }
                        .to_owned()
                            + k.into_iden().to_string().as_str()
                    })
                    .collect(),
            ),
            page: Some(value.page),
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

pub fn links_from_resource<I, T, R: Eq + Hash, C, Inc>(
    data: &[Resource<I, T, R>],
    opts: QueryOptions<C, Inc, I>,
    uri: &Uri,
) -> HashMap<LinkKey, String>
where
    I: for<'a> Deserialize<'a> + Serialize + Default + ToString + Clone,
    C: Eq + Hash + ColumnTrait,
    Inc: for<'a> Deserialize<'a> + Serialize + Clone,
{
    // TODO: a good way to find out if there is a page before
    // or if the next page is empty.

    let mut res = HashMap::new();
    if let Some(first) = data.first() {
        res.insert(LinkKey::First, first.id.to_string());
        if data.len() == opts.page.size as usize {
            let opts_clone = opts.clone();
            let prev_opts: RawQueryOptions<I> = QueryOptions {
                page: Page {
                    before: Some(first.id.clone()),
                    after: None,
                    ..opts_clone.page
                },
                ..opts_clone
            }
            .into();
            if let Ok(qs) = serde_qs::to_string(&prev_opts) {
                res.insert(LinkKey::Prev, uri.path().to_owned() + "?" + qs.as_str());
            }
        }
    }
    if let Some(last) = data.last() {
        res.insert(LinkKey::Last, last.id.to_string());
        if data.len() == opts.page.size as usize {
            let next_opts: RawQueryOptions<I> = QueryOptions {
                page: Page {
                    before: None,
                    after: Some(last.id.clone()),
                    ..opts.page
                },
                ..opts
            }
            .into();
            if let Ok(qs) = serde_qs::to_string(&next_opts) {
                res.insert(LinkKey::Next, uri.path().to_owned() + "?" + qs.as_str());
            }
        }
    }
    res
}
