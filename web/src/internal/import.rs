use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use base::setting::get_settings;
use eyre::Result;
use lazy_static::lazy_static;
use log::trace;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

use common::import;

lazy_static! {
    static ref JOBS: Arc<Mutex<HashMap<Uuid, Import>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Deserialize)]
pub struct ImportBegin {
    path: PathBuf,
    library: usize,
}

#[derive(Serialize, Clone)]
pub struct Import {
    id: Uuid,
    path: PathBuf,

    #[serde(flatten)]
    import: import::Import,
}

#[derive(Deserialize)]
pub enum ImportEdit {
    MbId(Uuid),
    Cover(usize),
}

#[axum_macros::debug_handler]
pub async fn begin(body: Json<ImportBegin>) -> Result<Json<Import>, StatusCode> {
    let path = get_settings()
        .map_err(|e| {
            trace!("Could not get settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .downloads
        .join(&body.path);
    let import = Import {
        id: Uuid::new_v4(),
        path: body.path.clone(),

        import: import::begin(body.library, &path).await.map_err(|e| {
            // TODO: better errors with json output
            trace!("Could not begin import: {}", e);
            StatusCode::BAD_REQUEST
        })?,
    };
    let mut imports = JOBS.lock().map_err(|e| {
        trace!("Could not lock imports table: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    imports.insert(import.id, import.clone());
    Ok(Json(import))
}

pub async fn edit(
    Path(job): Path<Uuid>,
    edit: Json<ImportEdit>,
) -> Result<Json<Import>, StatusCode> {
    let mut imports = JOBS.lock().map_err(|e| {
        trace!("Could not lock imports table: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mut import = imports
        .get(&job)
        .ok_or(StatusCode::NOT_FOUND)
        .map(|v| v.clone())?;
    // match edit.0 {
    //     ImportEdit::MbId(id) => import.mbid = id,
    //     ImportEdit::Cover(i) => import.cover = i,
    // }
    // imports.insert(job, import.clone());
    Ok(Json(import))
}

pub async fn run(Path(job): Path<Uuid>) -> Result<Json<()>, StatusCode> {
    Ok(Json(()))
}

pub async fn delete(Path(job): Path<Uuid>) -> Result<Json<()>, StatusCode> {
    Ok(Json(()))
}
