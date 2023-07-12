use axum::{extract::State, http::StatusCode};
use base::setting::get_settings;
use eyre::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    api::{
        extract::{Json, Path},
        internal::documents::{
            dedup, DirectoryAttributes, DirectoryMeta, DirectoryRelation, DirectoryResource,
            Included, InsertJobResource, JobAttributes, JobFilter, JobInclude, JobRelation,
            JobResource, ResourceType,
        },
        jsonapi::{
            links_from_resource, make_cursor, Document, DocumentData, Error, InsertDocument, Query,
            Related, Relation, Relationship, ResourceIdentifier,
        },
        AppState,
    },
    scheduling,
};
use common::import;

use super::{
    documents::{ImportInclude, ImportResource},
    downloads,
    jobs::job,
};

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

pub async fn begin(
    State(AppState(db)): State<AppState>,
    json_import: Json<InsertDocument<InsertJobResource>>,
) -> Result<Json<Document<ImportResource, ImportInclude>>, Error> {
    let body = json_import.inner();
    let settings = get_settings().map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not read settings".to_string(),
        detail: Some(err.into()),
    })?;
    let insert_imports = match body.data {
        DocumentData::Single(import) => vec![import],
        DocumentData::Multi(imports) => imports,
    };
    let mut imports = Vec::with_capacity(insert_imports.len());
    for import in insert_imports.into_iter() {
        let dir = downloads::abs_path(settings, import.attributes.path)?;
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
        imports.insert();
    }
    Ok(Json::new(Document {
        data: if imports.len() == 1 {
            DocumentData::Single(imports[0])
        } else {
            DocumentData::Multi(imports)
        },
        included: Vec::new(),
        links: HashMap::new(),
    }))
}

pub async fn get(job_path: Path<Uuid>) -> Result<Json<Import>, StatusCode> {
    let job = job_path.inner();
    let imports = JOBS.lock().await;
    imports
        .get(&job)
        .ok_or(StatusCode::NOT_FOUND)
        .map(|v| Json::new(v.clone()))
}

pub async fn edit(
    job_path: Path<Uuid>,
    json_edit: Json<ImportEdit>,
) -> Result<Json<Import>, StatusCode> {
    let job = job_path.inner();
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

pub async fn run(job_path: Path<Uuid>) -> Result<Json<()>, (StatusCode, Json<ImportError>)> {
    let job = job_path.inner();
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

pub async fn delete(job_path: Path<Uuid>) -> Result<Json<()>, StatusCode> {
    let job = job_path.inner();
    let mut imports = JOBS.lock().await;
    imports.remove(&job).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json::new(()))
}
