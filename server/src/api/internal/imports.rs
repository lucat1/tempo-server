use axum::{
    extract::{OriginalUri, Path as AxumPath, State},
    http::StatusCode,
};
use base::setting::get_settings;
use eyre::{eyre, Result};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, CursorTrait, DbErr, EntityTrait,
    IntoActiveModel, QueryOrder, TransactionTrait,
};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf};
use taskie_client::InsertTask;
use time::Duration;
use uuid::Uuid;

use crate::{
    api::{
        extract::{Json, Path},
        internal::{
            documents::{
                dedup, ImportAttributes, ImportFilter, ImportInclude, ImportResource, Included,
                InsertImportResource, JobInclude, ResourceType, UpdateImportCover,
            },
            downloads,
        },
        jsonapi::{
            links_from_resource, make_cursor, Document, DocumentData, Error, InsertOneDocument,
            Query, QueryOptions, UpdateOneDocument,
        },
        AppState,
    },
    import::{all_tracks, IntoInternal},
    tasks::{import, push, TaskName},
};

use super::documents::{UpdateImportAttributes, UpdateImportRelease, UpdateImportResource};

#[derive(Default)]
pub struct ImportRelated {
    directory: String,
}

pub fn entity_to_resource(entity: &entity::Import, related: &ImportRelated) -> ImportResource {
    ImportResource {
        id: entity.id,
        r#type: ResourceType::Import,
        attributes: ImportAttributes {
            source_release: entity.source_release.clone(),
            source_tracks: entity.source_tracks.0.clone(),
            started_at: entity.started_at,
            ended_at: entity.ended_at,
            selected_release: entity.selected_release,
            selected_cover: entity.selected_cover,
        },
        relationships: HashMap::new(),
        meta: None,
    }
}

pub fn entity_to_included(entity: &entity::Import, related: &ImportRelated) -> Included {
    Included::Import(entity_to_resource(entity, related))
}

pub async fn related<C>(
    _db: &C,
    entities: &[entity::Import],
    _light: bool,
) -> Result<Vec<ImportRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let mut result = Vec::new();
    for entity in entities.iter() {
        result.push(ImportRelated {
            directory: entity.directory.to_owned(),
        })
    }
    Ok(result)
}

pub async fn included<C>(
    _db: &C,
    _related: Vec<ImportRelated>,
    include: &[ImportInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    Ok(included)
}

pub async fn begin(
    State(AppState(db)): State<AppState>,
    json_import: Json<InsertOneDocument<InsertImportResource>>,
) -> Result<Json<Document<ImportResource, ImportInclude>>, Error> {
    let body = json_import.inner();
    let settings = get_settings().map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not read settings".to_string(),
        detail: Some(err.into()),
    })?;
    let decoded_path =
        urlencoding::decode(body.data.attributes.directory.as_str()).map_err(|err| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not decode directory path id".to_string(),
            detail: Some(err.into()),
        })?;
    let dir = downloads::abs_path(settings, Some(PathBuf::from(decoded_path.to_string())))?;
    tracing::info! {?dir, library = settings.library.name, "Importing folder"};
    let tracks = all_tracks(&settings.library, &dir)
        .await
        .map_err(|err| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not find all tracks in folder".to_string(),
            detail: Some(err.into()),
        })?;
    if tracks.is_empty() {
        return Err(Error {
            status: StatusCode::BAD_REQUEST,
            title: "Import folder does not contain any valid track files".to_string(),
            detail: None,
        });
    }

    let release: entity::InternalRelease = tracks.clone().into_internal();
    let tracks: Vec<entity::InternalTrack> =
        tracks.into_iter().map(|t| t.into_internal()).collect();

    // save the import in the db
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let dir = dir.to_string_lossy().to_string();
    let import = entity::ImportActive {
        id: ActiveValue::Set(Uuid::new_v4()),
        directory: ActiveValue::Set(dir.to_owned()),
        source_release: ActiveValue::Set(release),
        source_tracks: ActiveValue::Set(entity::InternalTracks(tracks)),

        artists: ActiveValue::Set(entity::import::Artists(Vec::new())),
        artist_credits: ActiveValue::Set(entity::import::ArtistCredits(Vec::new())),
        releases: ActiveValue::Set(entity::import::Releases(Vec::new())),
        mediums: ActiveValue::Set(entity::import::Mediums(Vec::new())),
        tracks: ActiveValue::Set(entity::import::Tracks(Vec::new())),
        artist_track_relations: ActiveValue::Set(entity::import::ArtistTrackRelations(Vec::new())),
        artist_credit_releases: ActiveValue::Set(entity::import::ArtistCreditReleases(Vec::new())),
        artist_credit_tracks: ActiveValue::Set(entity::import::ArtistCreditTracks(Vec::new())),
        covers: ActiveValue::Set(entity::import::Covers(Vec::new())),

        release_matches: ActiveValue::Set(entity::import::ReleaseMatches(HashMap::new())),
        cover_ratings: ActiveValue::Set(entity::import::CoverRatings(Vec::new())),
        selected_release: ActiveValue::NotSet,
        selected_cover: ActiveValue::NotSet,

        started_at: ActiveValue::Set(time::OffsetDateTime::now_utc()),
        ended_at: ActiveValue::NotSet,
    };
    let import = import.insert(&tx).await.map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't save the import structure".to_string(),
        detail: Some(err.into()),
    })?;
    tx.commit().await.map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't commit the transaction".to_string(),
        detail: Some(err.into()),
    })?;
    push(&[InsertTask {
        name: TaskName::ImportFetch,
        payload: Some(json!(import::fetch::Data(import.id))),
        depends_on: Vec::new(),
        duration: Duration::seconds(60),
    }])
    .await
    .map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not schedule import tasks".to_string(),
        detail: Some(err.into()),
    })?;

    let related = ImportRelated { directory: dir };
    let resource = entity_to_resource(&import, &related);

    Ok(Json::new(Document {
        data: DocumentData::Single(resource),
        included: Vec::new(),
        links: HashMap::new(),
    }))
}

async fn cover_refetch(import_id: Uuid) -> Result<()> {
    let fetch_task = push(&[InsertTask {
        name: TaskName::ImportFetchCovers,
        payload: Some(json!(import::fetch_covers::Data(import_id))),
        depends_on: Vec::new(),
        duration: Duration::seconds(360),
    }])
    .await?;
    push(&[InsertTask {
        name: TaskName::ImportRankCovers,
        payload: Some(json!(import::rank_covers::Data(import_id))),
        depends_on: vec![fetch_task
            .first()
            .ok_or(eyre!("Expected a task to have been queued"))?
            .id
            .clone()],
        duration: Duration::seconds(60),
    }])
    .await?;
    Ok(())
}

pub async fn edit(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ImportFilter, entity::ImportColumn, ImportInclude, Uuid>,
    import_path: Path<Uuid>,
    import_edit: Json<UpdateOneDocument<UpdateImportResource>>,
) -> Result<Json<Document<ImportResource, Included>>, Error> {
    let id = import_path.inner();
    let body = import_edit.inner();
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let import = entity::ImportEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried import".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Import not found".to_string(),
            detail: None,
        })?;
    let releases = import.releases.0.clone();
    let covers_len = import.covers.0.len();
    let mut import_active = import.into_active_model();
    match &body.data.attributes {
        UpdateImportAttributes::Release(UpdateImportRelease { selected_release }) => {
            if !releases.iter().any(|rel| rel.id == *selected_release) {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    title: "Cannot select a non-existant release".to_string(),
                    detail: None,
                });
            }
            import_active.selected_release = ActiveValue::Set(Some(*selected_release))
        }
        UpdateImportAttributes::Cover(UpdateImportCover { selected_cover }) => {
            if *selected_cover < 0 || *selected_cover >= (covers_len as i32) {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    title: "Cannot select a non-existant cover".to_string(),
                    detail: None,
                });
            }
            import_active.selected_cover = ActiveValue::Set(Some(*selected_cover))
        }
    }
    import_active.update(&tx).await.map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't update the import structure".to_string(),
        detail: Some(err.into()),
    })?;
    tx.commit().await.map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't commit the transaction".to_string(),
        detail: Some(err.into()),
    })?;
    if matches!(body.data.attributes, UpdateImportAttributes::Release(_)) {
        cover_refetch(id).await.map_err(|err| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not schedule cover refetches".to_string(),
            detail: Some(err.into()),
        })?;
    }

    self::import(State(AppState(db)), Query(opts), Path(AxumPath(id))).await
}

pub async fn imports(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ImportFilter, entity::ImportColumn, ImportInclude, Uuid>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<ImportResource, Included>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let mut imports_query = entity::ImportEntity::find();
    for (sort_key, sort_order) in opts.sort.iter() {
        imports_query = imports_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    let mut _imports_cursor = imports_query.cursor_by(entity::ArtistColumn::Id);
    let imports_cursor = make_cursor(&mut _imports_cursor, &opts.page);
    let imports = imports_cursor.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch imports page".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_imports = related(&tx, &imports, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch the related imports".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, import) in imports.iter().enumerate() {
        data.push(entity_to_resource(import, &related_to_imports[i]));
    }
    let included = included(&tx, related_to_imports, &opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        links: links_from_resource(&data, opts, &uri),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

async fn fetch_import_data<C>(
    db: &C,
    import: entity::Import,
    opts: &QueryOptions<ImportFilter, entity::ImportColumn, ImportInclude, Uuid>,
) -> Result<Json<Document<ImportResource, Included>>, Error>
where
    C: ConnectionTrait,
{
    let related_to_imports = related(db, &[import.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the related imports".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = ImportRelated::default();
    let related = related_to_imports.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&import, related);
    let included = included(db, related_to_imports, &opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        data: DocumentData::Single(data),
        included: dedup(included),
        links: HashMap::new(),
    }))
}

pub async fn import(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ImportFilter, entity::ImportColumn, ImportInclude, Uuid>,
    import_path: Path<Uuid>,
) -> Result<Json<Document<ImportResource, Included>>, Error> {
    let id = import_path.inner();
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let import = entity::ImportEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried import".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Import not found".to_string(),
            detail: None,
        })?;
    fetch_import_data(&tx, import, &opts).await
}

// pub async fn run(job_path: Path<Uuid>) -> Result<Json<()>, (StatusCode, Json<ImportError>)> {
//     let job = job_path.inner();
//     let mut imports = JOBS.lock().await;
//     let import = imports.remove(&job).ok_or((
//         StatusCode::NOT_FOUND,
//         Json::new(ImportError {
//             message: "".to_string(),
//         }),
//     ))?;
//     import::run(import.import).await.map_err(|e| {
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json::new(ImportError {
//                 message: e.to_string(),
//             }),
//         )
//     })?;
//     Ok(Json::new(()))
// }

// pub async fn delete(job_path: Path<Uuid>) -> Result<Json<()>, StatusCode> {
//     let job = job_path.inner();
//     let mut imports = JOBS.lock().await;
//     imports.remove(&job).ok_or(StatusCode::NOT_FOUND)?;
//     Ok(Json::new(()))
// }
