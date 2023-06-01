use axum::extract::Query;
use axum::http::StatusCode;
use eyre::Result;
use fs_extra::dir::get_size;
use serde::{Deserialize, Serialize};
use std::{fs::read_dir, path::PathBuf};

use crate::api::extract::Json;
use base::setting::get_settings;

#[derive(Deserialize)]
pub struct ListRequest {
    path: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct List {
    name: String,
    entries: Vec<Entry>,
}

#[derive(Serialize)]
pub struct Entry {
    name: String,
    path: PathBuf,
    r#type: EntryType,
    size: u64,
}
#[derive(Serialize)]
pub enum EntryType {
    File = 0,
    Directory = 1,
}

pub async fn list(query: Query<ListRequest>) -> Result<Json<List>, StatusCode> {
    let root_path = get_settings()
        .map_err(|error| {
            tracing::warn! {%error, "Could not get settings"};
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .downloads
        .clone();
    let path = query
        .path
        .as_ref()
        .map(|sub| {
            if sub.is_relative() {
                root_path.join(sub)
            } else {
                root_path.clone()
            }
        })
        .unwrap_or_else(|| root_path.clone());
    let raw_files = read_dir(&path).map_err(|error| {
        tracing::warn! {%error, ?path, "Could not red directory"};
        StatusCode::BAD_REQUEST
    })?;
    let files: Vec<Entry> = raw_files
        .filter_map(|f| f.ok())
        .filter_map(|f| -> Option<Entry> {
            Some(Entry {
                name: f.file_name().to_string_lossy().to_string(),
                path: f.path().strip_prefix(&root_path).ok()?.to_path_buf(),
                r#type: if f.metadata().ok()?.is_file() {
                    EntryType::File
                } else {
                    EntryType::Directory
                },
                size: get_size(f.path()).ok()?,
            })
        })
        .collect();
    Ok(Json::new(List {
        name: path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or("/".to_string()),
        entries: files,
    }))
}
