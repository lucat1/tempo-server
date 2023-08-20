use base::setting::JobType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::api::jsonapi::{InsertResource, Resource, UpdateResource};
use entity::{InternalRelease, InternalTrack};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Directory,
    Job,
    Task,
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
pub struct JobAttributes {
    pub title: String,
    pub description: Option<String>,
    #[serde(with = "time::serde::iso8601")]
    pub scheduled_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct InsertJobAttributes {
    pub r#type: JobType,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobRelation {
    Tasks,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobInclude {
    Tasks,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobMeta {}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobFilter {}

pub type JobResource = Resource<ResourceType, i64, JobAttributes, JobRelation, JobMeta>;
pub type InsertJobResource =
    InsertResource<ResourceType, InsertJobAttributes, JobRelation, JobMeta>;

#[derive(Serialize, Deserialize)]
pub struct TaskAttributes {
    pub data: serde_json::Value,
    pub description: Option<String>,

    #[serde(with = "time::serde::iso8601")]
    pub scheduled_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::iso8601::option")]
    pub ended_at: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskRelation {
    Job,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskInclude {
    Job,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskMeta {}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskFilter {}

pub type TaskResource = Resource<ResourceType, i64, TaskAttributes, TaskRelation, TaskMeta>;

#[derive(Serialize, Deserialize)]
pub struct ImportAttributes {
    pub source_release: InternalRelease,
    pub source_tracks: Vec<InternalTrack>,

    #[serde(with = "time::serde::iso8601")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option")]
    pub ended_at: Option<OffsetDateTime>,

    pub selected_release: Option<Uuid>,
    pub selected_cover: Option<i32>,
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
    Job,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub enum ImportInclude {
    #[serde(rename = "job")]
    Job,
    #[serde(rename = "job.tasks")]
    JobTasks,
}

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
    Job(JobResource),
    Task(TaskResource),
    Import(ImportResource),
}

#[derive(PartialEq)]
enum Identifier {
    Directory(String),
    Task(i64),
    Job(i64),
    Import(Uuid),
}

pub fn dedup(mut included: Vec<Included>) -> Vec<Included> {
    included.sort_unstable_by(|_a, _b| match (_a, _b) {
        (Included::Directory(a), Included::Directory(b)) => a.id.cmp(&b.id),
        (Included::Job(a), Included::Job(b)) => a.id.cmp(&b.id),
        (Included::Task(a), Included::Task(b)) => a.id.cmp(&b.id),
        (Included::Import(a), Included::Import(b)) => a.id.cmp(&b.id),
        (_, _) => std::cmp::Ordering::Less,
    });
    included.dedup_by_key(|e| match e {
        Included::Directory(e) => Identifier::Directory(e.id.to_owned()),
        Included::Job(e) => Identifier::Job(e.id),
        Included::Task(e) => Identifier::Task(e.id),
        Included::Import(e) => Identifier::Import(e.id),
    });
    included
}
