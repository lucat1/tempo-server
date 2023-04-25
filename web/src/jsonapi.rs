use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::Order;
use serde::{Deserialize, Serialize};
use std::{cmp::Eq, collections::HashMap, error::Error as StdError, hash::Hash};
use uuid::Uuid;

use super::documents::{
    ArtistAttributes, ArtistCreditAttributes, ArtistCreditRelation, ArtistRelation,
    MediumAttributes, MediumRelation, ReleaseAttributes, ReleaseRelation, TrackAttributes,
    TrackRelation,
};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Artist,
    ArtistCredit,
    Track,
    Medium,
    Release,
}

pub type ArtistResource = Resource<Uuid, ArtistAttributes, ArtistRelation>;
pub type ArtistCreditResource = Resource<String, ArtistCreditAttributes, ArtistCreditRelation>;
pub type TrackResource = Resource<Uuid, TrackAttributes, TrackRelation>;
pub type MediumResource = Resource<Uuid, MediumAttributes, MediumRelation>;
pub type ReleaseResource = Resource<Uuid, ReleaseAttributes, ReleaseRelation>;

#[derive(Serialize)]
#[serde(untagged)]
pub enum Included {
    Artist(ArtistResource),
    ArtistCredit(ArtistCreditResource),
    Track(TrackResource),
    Medium(MediumResource),
    Release(ReleaseResource),
}

#[derive(Serialize)]
pub struct Document<R> {
    pub data: DocumentData<R>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub included: Vec<Included>,
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
    Artist(RelationData<Uuid>),
    ArtistCredit(RelationData<String>),
    Track(RelationData<Uuid>),
    Medium(RelationData<Uuid>),
    Release(RelationData<Uuid>),
}

#[derive(Serialize)]
pub struct RelationData<I> {
    pub r#type: ResourceType,
    pub id: I,
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

#[derive(Deserialize)]
pub struct RawQueryOptions {
    include: Option<String>,
    filter: Option<HashMap<String, String>>,
    sort: Option<String>,
}

#[derive(Debug)]
pub struct QueryOptions<C: Eq + Hash + TryFrom<String>> {
    pub include: Vec<String>,
    pub filter: HashMap<C, String>,
    pub sort: HashMap<C, Order>,
}

pub struct Query<C: Eq + Hash + TryFrom<String>>(pub QueryOptions<C>);

#[async_trait]
impl<S, C> FromRequestParts<S> for Query<C>
where
    S: Send + Sync,
    C: Eq + Hash + TryFrom<String>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query();
        match query {
            Some(qs) => {
                let raw_opts: RawQueryOptions = serde_qs::from_str(qs)
                    .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

                let parse_key = |k: &str| -> Option<C> { k.to_owned().try_into().ok() };

                let opts = QueryOptions {
                    filter: raw_opts
                        .filter
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|(k, v)| -> Option<(C, String)> {
                            Some((parse_key(k.as_str())?, v))
                        })
                        .collect(),
                    include: raw_opts
                        .include
                        .as_ref()
                        .map(|s| s.split(",").map(|p| p.to_owned()).collect())
                        .unwrap_or(Vec::new()),
                    sort: raw_opts
                        .sort
                        .map(|s| {
                            s.split(",")
                                .filter_map(|p| -> Option<(C, Order)> {
                                    if p.starts_with("-") {
                                        Some((parse_key(&p[1..])?, Order::Desc))
                                    } else {
                                        Some((parse_key(p)?, Order::Asc))
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or(HashMap::new()),
                };
                Ok(Query(opts))
            }
            None => Ok(Self(QueryOptions {
                include: Vec::new(),
                filter: HashMap::new(),
                sort: HashMap::new(),
            })),
        }
    }
}
