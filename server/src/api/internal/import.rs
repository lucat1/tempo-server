use axum::extract::Path;
use axum::http::StatusCode;
use base::setting::get_settings;
use eyre::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::api::extract::Json;
use common::import;

lazy_static! {
    static ref JOBS: Arc<Mutex<HashMap<Uuid, Import>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Deserialize)]
pub struct ImportBegin {
    path: PathBuf,
}

#[derive(Serialize, Clone)]
pub struct Import {
    id: Uuid,
    path: PathBuf,

    #[serde(flatten)]
    pub import: import::Import,
}

#[derive(Deserialize)]
pub enum ImportEdit {
    MbId(Uuid),
    Cover(usize),
}

#[derive(Serialize, Clone)]
pub struct ImportError {
    message: String,
}

pub async fn begin(json_body: Json<ImportBegin>) -> Result<Json<Import>, StatusCode> {
    let body = json_body.inner();
    let path = get_settings()
        .map_err(|error| {
            tracing::warn! { %error, "Could not get settings" };
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .downloads
        .join(&body.path);
    let import = Import {
        id: Uuid::new_v4(),
        path: body.path.clone(),

        import: import::begin(&path).await.map_err(|error| {
            // TODO: better errors with json output
            tracing::warn! { %error, "Could not begin import" };
            StatusCode::BAD_REQUEST
        })?,
    };
    let mut imports = JOBS.lock().await;
    imports.insert(import.id, import.clone());
    Ok(Json::new(import))
}

pub async fn get(Path(job): Path<Uuid>) -> Result<Json<Import>, StatusCode> {
    let imports = JOBS.lock().await;
    imports
        .get(&job)
        .ok_or(StatusCode::NOT_FOUND)
        .map(|v| Json::new(v.clone()))
}

pub async fn edit(
    Path(job): Path<Uuid>,
    json_edit: Json<ImportEdit>,
) -> Result<Json<Import>, StatusCode> {
    let edit = json_edit.inner();
    let mut imports = JOBS.lock().await;
    let mut import = imports
        .get(&job)
        .ok_or(StatusCode::NOT_FOUND)
        .map(|v| v.clone())?;
    match edit {
        ImportEdit::MbId(id) => {
            if !import
                .import
                .search_results
                .iter()
                .any(|r| r.search_result.0.release.id == id)
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            // TODO: the MbId has been changed, update the cover options
            import.import.selected.0 = id
        }
        ImportEdit::Cover(i) => {
            if i >= import.import.covers.len() {
                return Err(StatusCode::BAD_REQUEST);
            }
            import.import.selected.1 = Some(i)
        }
    }
    imports.insert(job, import.clone());
    Ok(Json::new(import))
}

pub async fn run(Path(job): Path<Uuid>) -> Result<Json<()>, (StatusCode, Json<ImportError>)> {
    let mut imports = JOBS.lock().await;
    let import = imports.remove(&job).ok_or((
        StatusCode::NOT_FOUND,
        Json::new(ImportError {
            message: "".to_string(),
        }),
    ))?;
    import::run(import.import).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json::new(ImportError {
                message: e.to_string(),
            }),
        )
    })?;
    Ok(Json::new(()))
}

pub async fn delete(Path(job): Path<Uuid>) -> Result<Json<()>, StatusCode> {
    let mut imports = JOBS.lock().await;
    imports.remove(&job).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json::new(()))
}
