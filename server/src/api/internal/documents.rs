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
    Task,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobMeta {}

pub type JobResource = Resource<ResourceType, i64, JobAttributes, JobRelation, JobMeta>;
pub type InsertJobResource =
    InsertResource<ResourceType, InsertJobAttributes, JobRelation, JobMeta>;
