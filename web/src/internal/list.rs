use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use base::setting::get_settings;
use eyre::Result;
use log::trace;
use serde::{Deserialize, Serialize};
use std::{fs::read_dir, path::PathBuf};

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
}
#[derive(Serialize)]
pub enum EntryType {
    File = 0,
    Directory = 1,
}

pub async fn list(query: Query<ListRequest>) -> Result<Json<List>, StatusCode> {
    let root_path = get_settings()
        .map_err(|e| {
            trace!("Could not get settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .downloads
        .clone();
    let mut path = root_path.clone();
    if let Some(subpath) = &query.path {
        if subpath.is_relative() {
            path = path.join(subpath);
        }
    }
    let raw_files = read_dir(&path).map_err(|e| {
        trace!("Could not red directory: {:?}: {}", path, e);
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
            })
        })
        .collect();
    Ok(Json(List {
        name: path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or("/".to_string()),
        entries: files,
    }))
}
