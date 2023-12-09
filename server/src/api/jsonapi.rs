use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use eyre::ErrReport;
use itertools::Itertools;
use sea_orm::{
    sea_query::{IntoIden, IntoValueTuple},
    ColumnTrait, Cursor, Order, SelectorTrait,
};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{cmp::Eq, collections::HashMap, default::Default, error::Error as StdError, hash::Hash};
use uuid::Uuid;

use entity::ConnectionProvider;

pub static DEFAULT_PAGE_SIZE: u32 = 10;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LinkKey {
    Prev,
    Next,
    First,
    Last,
}

#[derive(Serialize, Deserialize)]
pub struct Document<R, I> {
    pub data: DocumentData<R>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub included: Vec<I>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub links: HashMap<LinkKey, String>,
}

#[derive(Serialize, Deserialize)]
pub struct InsertDocument<R> {
    pub data: DocumentData<R>,
}

#[derive(Serialize, Deserialize)]
pub struct InsertOneDocument<R> {
    pub data: R,
}

#[derive(Serialize, Deserialize)]
pub struct InsertOneRelation<R> {
    pub data: R,
}

#[derive(Serialize, Deserialize)]
pub struct InsertManyRelation<R> {
    pub data: Vec<R>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateDocument<R> {
    pub data: DocumentData<R>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateOneDocument<R> {
    pub data: R,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocumentData<R> {
    Single(R),
    Multi(Vec<R>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resource<RT, I, T, R: Eq + Hash, M> {
    pub r#type: RT,
    pub id: I,
    pub attributes: T,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    pub relationships: HashMap<R, Relationship<RT, M>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<M>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertResource<RT, T, R: Eq + Hash, M> {
    pub r#type: RT,
    pub attributes: T,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    pub relationships: HashMap<R, Relationship<RT, M>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<M>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateResource<RT, I, T, R: Eq + Hash, M> {
    pub r#type: RT,
    pub id: I,
    pub attributes: T,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    pub relationships: HashMap<R, Relationship<RT, M>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<M>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Relationship<RT, M> {
    pub data: Relation<RT, M>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Relation<RT, M> {
    Single(Related<RT, M>),
    Multi(Vec<Related<RT, M>>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Related<RT, M> {
    Uuid(ResourceIdentifier<RT, Uuid, M>),
    String(ResourceIdentifier<RT, String, M>),
    Int(ResourceIdentifier<RT, i64, M>),
    ConnectionProvider(ResourceIdentifier<RT, ConnectionProvider, M>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceIdentifier<RT, I, M> {
    pub r#type: RT,
    pub id: I,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<M>,
}

pub struct Error {
    pub status: StatusCode,
    pub title: String,
    pub detail: Option<Box<dyn StdError>>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.title)
    }
}

// TODO: remove once we get proper error handling done
impl From<ErrReport> for Error {
    fn from(err: ErrReport) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: err.to_string(),
            detail: Some(err.into()),
        }
    }
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
pub struct RawQueryOptions<F: Eq + Hash, Id: Default> {
    include: Option<String>,
    filter: Option<HashMap<F, String>>,
    sort: Option<String>,
    page: Option<Page<Id>>,
}

#[derive(Debug, Clone)]
pub struct QueryOptions<
    F: Eq + Hash,
    C: Eq + Hash,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
> {
    pub include: Vec<I>,
    pub filter: HashMap<F, String>,
    pub sort: HashMap<C, Order>,
    pub page: Page<Id>,
}

pub struct Query<
    F: Eq + Hash,
    C: Eq + Hash,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
>(pub QueryOptions<F, C, I, Id>);

#[async_trait]
impl<S, F, C, I, Id> FromRequestParts<S> for Query<F, C, I, Id>
where
    S: Send + Sync,
    F: Eq + Hash + for<'a> Deserialize<'a>,
    C: Eq + Hash + ColumnTrait,
    I: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Default,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query();
        match query {
            Some(qs) => {
                let raw_opts: RawQueryOptions<F, Id> = serde_qs::from_str(qs)
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
                    filter: raw_opts.filter.unwrap_or_default(),
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
                        .unwrap_or_default(),
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

impl<F, C, I, Id> From<QueryOptions<F, C, I, Id>> for RawQueryOptions<F, Id>
where
    F: Eq + Hash + for<'a> Deserialize<'a> + Serialize,
    C: Eq + Hash + ColumnTrait,
    I: for<'a> Deserialize<'a> + Serialize,
    Id: for<'a> Deserialize<'a> + Default,
{
    fn from(value: QueryOptions<F, C, I, Id>) -> Self {
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
            filter: if value.filter.is_empty() {
                None
            } else {
                Some(value.filter)
            },
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

pub fn make_cursor<'a, S, Id>(mut cursor: &'a mut Cursor<S>, page: &Page<Id>) -> &'a mut Cursor<S>
where
    S: SelectorTrait,
    Id: Default + IntoValueTuple + Clone,
{
    if let Some(before) = page.before.clone() {
        cursor = cursor.before(before);
    }
    if let Some(after) = page.after.clone() {
        cursor = cursor.after(after);
    }
    cursor = match (page.after.clone(), page.before.clone()) {
        (None, Some(_)) => cursor.last(page.size.into()),
        // Also matches (Some(_), None), which means "everything all after `after`"
        (_, _) => cursor.first(page.size.into()),
    };
    cursor
}

pub fn links_from_resource<RT, I, T, R: Eq + Hash, M, C, F, Inc>(
    data: &[Resource<RT, I, T, R, M>],
    opts: QueryOptions<F, C, Inc, I>,
    uri: &Uri,
) -> HashMap<LinkKey, String>
where
    I: for<'a> Deserialize<'a> + Serialize + Default + ToString + Clone,
    F: Eq + Hash + for<'a> Deserialize<'a> + Serialize + Clone,
    C: Eq + Hash + Clone + ColumnTrait,
    Inc: for<'a> Deserialize<'a> + Serialize + Clone,
{
    // TODO: a good way to find out if there is a page before
    // or if the next page is empty.

    let mut res = HashMap::new();
    if let Some(first) = data.first() {
        res.insert(LinkKey::First, first.id.to_string());
        if data.len() == opts.page.size as usize {
            let opts_clone = opts.clone();
            let prev_opts: RawQueryOptions<F, I> = QueryOptions {
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
            let next_opts: RawQueryOptions<F, I> = QueryOptions {
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
