use axum::{
    extract::{OriginalUri, State},
    http::StatusCode,
};
use base::setting::get_settings;
use eyre::Result;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, CursorTrait, DbErr,
    EntityTrait, LoaderTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

use crate::{
    api::{
        extract::{Json, Path},
        internal::{
            documents::{
                dedup, DirectoryAttributes, DirectoryMeta, DirectoryRelation, DirectoryResource,
                ImportAttributes, ImportFilter, ImportInclude, ImportRelation, ImportResource,
                Included, InsertImportResource, InsertJobResource, JobAttributes, JobFilter,
                JobInclude, JobRelation, JobResource, ResourceType,
            },
            downloads, jobs,
            jobs::job,
        },
        jsonapi::{
            links_from_resource, make_cursor, Document, DocumentData, Error, InsertOneDocument,
            Query, Related, Relation, Relationship, ResourceIdentifier,
        },
        AppState,
    },
    import::{all_tracks, IntoInternal},
    scheduling,
    tasks::{import::ImportFetch, TaskData},
};

#[derive(Default)]
pub struct ImportRelated {
    job: Option<entity::Job>,
    directory: String,
}

pub fn entity_to_resource(entity: &entity::Import, related: &ImportRelated) -> ImportResource {
    let mut relationships = HashMap::new();
    if let Some(job) = related.job.as_ref() {
        relationships.insert(
            ImportRelation::Job,
            Relationship {
                data: Relation::Single(Related::Int(ResourceIdentifier {
                    r#type: ResourceType::Job,
                    id: job.id,
                    meta: None,
                })),
            },
        );
    }
    ImportResource {
        id: entity.id,
        r#type: ResourceType::Import,
        attributes: ImportAttributes {
            source_release: entity.source_release.clone(),
            source_tracks: entity.source_tracks.0.clone(),
            started_at: entity.started_at,
            ended_at: entity.ended_at,
        },
        relationships,
        meta: None,
    }
}

pub fn entity_to_included(entity: &entity::Import, related: &ImportRelated) -> Included {
    Included::Import(entity_to_resource(entity, related))
}

pub async fn related<C>(
    db: &C,
    entities: &[entity::Import],
    _light: bool,
) -> Result<Vec<ImportRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let mut result = Vec::new();
    let jobs = entities.load_one(entity::JobEntity, db).await?;
    for (i, job) in jobs.into_iter().enumerate() {
        result.push(ImportRelated {
            job,
            directory: entities[i].directory.to_owned(),
        })
    }
    Ok(result)
}

fn map_to_jobs_include(include: &[ImportInclude]) -> Vec<JobInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            ImportInclude::JobTasks => Some(JobInclude::Tasks),
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    related: Vec<ImportRelated>,
    include: &[ImportInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&ImportInclude::Job) {
        let mut cond = Condition::any();
        for rel in related.iter() {
            if let Some(job) = &rel.job {
                cond = cond.add(ColumnTrait::eq(&entity::JobColumn::Id, job.id));
            }
        }
        let jobs = entity::JobEntity::find().filter(cond).all(db).await?;
        let jobs_related = jobs::related(db, &jobs, true).await?;
        for (i, job) in jobs.iter().enumerate() {
            included.push(jobs::entity_to_included(job, &jobs_related[i]))
        }
        let jobs_included = map_to_jobs_include(include);
        included.extend(jobs::included(db, jobs_related, &jobs_included).await?);
    }
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

    // save the import in the db, alongside its job and tasks
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let job = entity::JobActive {
        title: ActiveValue::Set(format!("Import {}", dir.to_string_lossy())),
        description: ActiveValue::Set(Some(format!(
            "Import the album at {}, interpreted as {} - {}",
            dir.to_string_lossy(),
            release.artists.join(","),
            release.title
        ))),
        scheduled_at: ActiveValue::Set(time::OffsetDateTime::now_utc()),
        ..Default::default()
    };
    let job = job.insert(&tx).await.map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't save the import job".to_string(),
        detail: Some(err.into()),
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

        started_at: ActiveValue::Set(time::OffsetDateTime::now_utc()),
        ended_at: ActiveValue::NotSet,
        job: ActiveValue::Set(job.id),
    };
    let import = import.insert(&tx).await.map_err(|err| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't save the import structure".to_string(),
        detail: Some(err.into()),
    })?;
    let tasks = vec![TaskData::ImportFetch(ImportFetch(import.id))];
    scheduling::schedule_tasks(tx, job.id, tasks)
        .await
        .map_err(|err| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not schedule import tasks".to_string(),
            detail: Some(err.into()),
        })?;

    let related = ImportRelated {
        job: Some(job),
        directory: dir,
    };
    let resource = entity_to_resource(&import, &related);

    Ok(Json::new(Document {
        data: DocumentData::Single(resource),
        included: Vec::new(),
        links: HashMap::new(),
    }))
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

pub async fn import(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ImportFilter, entity::ImportColumn, ImportInclude, i64>,
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
    let related_to_imports = related(&tx, &[import.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the related imports".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = ImportRelated::default();
    let related = related_to_imports.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&import, related);
    let included = included(&tx, related_to_imports, &opts.include)
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

// pub async fn edit(
//     job_path: Path<Uuid>,
//     json_edit: Json<ImportEdit>,
// ) -> Result<Json<Import>, StatusCode> {
//     let job = job_path.inner();
//     let edit = json_edit.inner();
//     let mut imports = JOBS.lock().await;
//     let mut import = imports
//         .get(&job)
//         .ok_or(StatusCode::NOT_FOUND)
//         .map(|v| v.clone())?;
//     match edit {
//         ImportEdit::MbId(id) => {
//             if !import
//                 .import
//                 .search_results
//                 .iter()
//                 .any(|r| r.search_result.0.release.id == id)
//             {
//                 return Err(StatusCode::BAD_REQUEST);
//             }
//             // TODO: the MbId has been changed, update the cover options
//             import.import.selected.0 = id
//         }
//         ImportEdit::Cover(i) => {
//             if i >= import.import.covers.len() {
//                 return Err(StatusCode::BAD_REQUEST);
//             }
//             import.import.selected.1 = Some(i)
//         }
//     }
//     imports.insert(job, import.clone());
//     Ok(Json::new(import))
// }
//
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
//
// pub async fn delete(job_path: Path<Uuid>) -> Result<Json<()>, StatusCode> {
//     let job = job_path.inner();
//     let mut imports = JOBS.lock().await;
//     imports.remove(&job).ok_or(StatusCode::NOT_FOUND)?;
//     Ok(Json::new(()))
// }
