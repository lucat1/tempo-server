use base::setting::JobType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;

use crate::api::jsonapi::{InsertResource, Resource};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Directory,
    Job,
    Task,
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

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskMeta {}

pub type TaskResource = Resource<ResourceType, i64, TaskAttributes, TaskRelation, TaskMeta>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Included {
    Task(TaskResource),
}

#[derive(PartialEq)]
enum Identifier {
    Task(i64),
}

pub fn dedup(mut included: Vec<Included>) -> Vec<Included> {
    included.sort_unstable_by(|_a, _b| match (_a, _b) {
        (Included::Task(a), Included::Task(b)) => a.id.cmp(&b.id),
        // (_, _) => std::cmp::Ordering::Less,
    });
    included.dedup_by_key(|e| match e {
        Included::Task(e) => Identifier::Task(e.id),
    });
    included
}
