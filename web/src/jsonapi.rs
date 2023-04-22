use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::{collections::HashMap, error::Error as StdError};
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
