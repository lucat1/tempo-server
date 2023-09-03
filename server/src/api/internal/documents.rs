use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::api::jsonapi::{InsertResource, Resource, UpdateResource};
use entity::{InternalRelease, InternalTrack};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ResourceType {
    Internal(InternalResourceType),
    Tempo(crate::api::documents::ResourceType),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum InternalResourceType {
    Directory,
    Import,
}

#[derive(Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Serialize, Deserialize)]
pub struct DirectoryAttributes {
    pub name: String,
    pub path: PathBuf,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryRelation {
    Directories,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryMeta {}

pub type DirectoryResource =
    Resource<ResourceType, String, DirectoryAttributes, DirectoryRelation, DirectoryMeta>;

#[derive(Serialize, Deserialize)]
pub struct ImportAttributes {
    pub source_release: InternalRelease,
    pub source_tracks: Vec<InternalTrack>,

    pub covers: Vec<entity::import::Cover>,
    pub release_matches: HashMap<Uuid, entity::import::ReleaseRating>,
    pub cover_ratings: Vec<f32>,

    pub selected_release: Option<Uuid>,
    pub selected_cover: Option<i32>,

    #[serde(with = "time::serde::iso8601")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option")]
    pub ended_at: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize)]
pub struct InsertImportAttributes {
    pub directory: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateImportRelease {
    pub selected_release: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateImportCover {
    pub selected_cover: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateImportAttributes {
    Release(UpdateImportRelease),
    Cover(UpdateImportCover),
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportRelation {
    Directory,
    Releases,
    Mediums,
    Tracks,
    Artists,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub enum ImportInclude {}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportMeta {}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportFilter {}

pub type ImportResource =
    Resource<ResourceType, Uuid, ImportAttributes, ImportRelation, ImportMeta>;
pub type InsertImportResource =
    InsertResource<ResourceType, InsertImportAttributes, ImportRelation, ImportMeta>;
pub type UpdateImportResource =
    UpdateResource<ResourceType, Uuid, UpdateImportAttributes, ImportRelation, ImportMeta>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Included {
    Directory(DirectoryResource),
    Import(ImportResource),

    TempoInclude(crate::api::documents::Included),
}

impl PartialEq for Included {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Included::Directory(a), Included::Directory(b)) => a.id == b.id,
            (Included::Import(a), Included::Import(b)) => a.id == b.id,
            (_, _) => false,
        }
    }
}
impl Eq for Included {}

impl std::cmp::PartialOrd for Included {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Included::Directory(a), Included::Directory(b)) => a.id.partial_cmp(&b.id),
            (Included::Import(a), Included::Import(b)) => a.id.partial_cmp(&b.id),
            (_, _) => None,
        }
    }
}

impl std::cmp::Ord for Included {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Included::Directory(a), Included::Directory(b)) => a.id.cmp(&b.id),
            (Included::Import(a), Included::Import(b)) => a.id.cmp(&b.id),
            (_, _) => std::cmp::Ordering::Less,
        }
    }
}
