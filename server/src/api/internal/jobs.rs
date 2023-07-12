use axum::{
    extract::{OriginalUri, State},
    http::StatusCode,
};
use eyre::Result;
use sea_orm::{
    ConnectionTrait, CursorTrait, DbErr, EntityTrait, LoaderTrait, QueryOrder, TransactionTrait,
};
use std::collections::HashMap;

use crate::{
    api::{
        extract::{Json, Path},
        internal::documents::{
            dedup, Included, InsertJobResource, JobAttributes, JobFilter, JobInclude, JobRelation,
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

use super::tasks;

#[derive(Default)]
pub struct JobRelated {
    tasks: Vec<entity::Task>,
}

pub fn entity_to_resource(entity: &entity::Job, related: &JobRelated) -> JobResource {
    JobResource {
        id: entity.id,
        r#type: ResourceType::Job,
        attributes: JobAttributes {
            title: entity.title.to_owned(),
            description: entity.description.to_owned(),
            scheduled_at: entity.scheduled_at,
        },
        relationships: [(
            JobRelation::Tasks,
            Relationship {
                data: Relation::Multi(
                    related
                        .tasks
                        .iter()
                        .map(|t| {
                            Related::Int(ResourceIdentifier {
                                r#type: ResourceType::Task,
                                id: t.id,
                                meta: None,
                            })
                        })
                        .collect(),
                ),
            },
        )]
        .into(),
        meta: None,
    }
}

pub fn entity_to_included(entity: &entity::Job, related: &JobRelated) -> Included {
    Included::Job(entity_to_resource(entity, related))
}

pub async fn related<C>(
    db: &C,
    entities: &[entity::Job],
    _light: bool,
) -> Result<Vec<JobRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let mut result = Vec::new();
    let jobs_tasks = entities.load_many(entity::TaskEntity, db).await?;
    for tasks in jobs_tasks.into_iter() {
        result.push(JobRelated { tasks })
    }
    Ok(result)
}

pub async fn included<C>(
    db: &C,
    related: Vec<JobRelated>,
    include: &[JobInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&JobInclude::Tasks) {
        let tasks = related
            .iter()
            .flat_map(|rel| rel.tasks.to_owned())
            .collect::<Vec<_>>();
        let tasks_related = tasks::related(db, &tasks, true).await?;
        for (i, task) in tasks.iter().enumerate() {
            included.push(tasks::entity_to_included(task, &tasks_related[i]))
        }
    }
    Ok(included)
}

pub async fn schedule(
    State(AppState(db)): State<AppState>,
    json_jobs: Json<InsertDocument<InsertJobResource>>,
) -> Result<Json<Document<JobResource, Included>>, Error> {
    let document_jobs = json_jobs.inner();
    let jobs = match document_jobs.data {
        DocumentData::Single(r) => vec![r],
        DocumentData::Multi(rs) => rs,
    };
    let mut queued_jobs = Vec::new();
    for job in jobs.iter() {
        let tx = db.begin().await.map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Couldn't begin database transaction".to_string(),
            detail: Some(e.into()),
        })?;
        let job = scheduling::trigger_job(tx, &job.attributes.r#type)
            .await
            .map_err(|e| Error {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                title: "Could not schedule job".to_string(),
                detail: Some(e.into()),
            })?;
        queued_jobs.push(job);
    }
    let related_to_jobs = related(&db, &queued_jobs, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch the related jobs".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, job) in queued_jobs.iter().enumerate() {
        data.push(entity_to_resource(job, &related_to_jobs[i]));
    }
    let included = included(&db, related_to_jobs, &[JobInclude::Tasks])
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        data: DocumentData::Multi(data),
        included: dedup(included),
        links: HashMap::new(),
    }))
}

pub async fn jobs(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<JobFilter, entity::JobColumn, JobInclude, i64>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<JobResource, Included>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let mut jobs_query = entity::JobEntity::find();
    for (sort_key, sort_order) in opts.sort.iter() {
        jobs_query = jobs_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    let mut _jobs_cursor = jobs_query.cursor_by(entity::ArtistColumn::Id);
    let jobs_cursor = make_cursor(&mut _jobs_cursor, &opts.page);
    let jobs = jobs_cursor.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch jobs page".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_jobs = related(&tx, &jobs, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch the related jobs".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, job) in jobs.iter().enumerate() {
        data.push(entity_to_resource(job, &related_to_jobs[i]));
    }
    let included = included(&tx, related_to_jobs, &opts.include)
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

pub async fn job(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<JobFilter, entity::JobColumn, JobInclude, i64>,
    job_path: Path<i64>,
) -> Result<Json<Document<JobResource, Included>>, Error> {
    let id = job_path.inner();
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let job = entity::JobEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried job".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Job not found".to_string(),
            detail: None,
        })?;
    let related_to_jobs = related(&tx, &[job.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the related jobs".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = JobRelated::default();
    let related = related_to_jobs.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&job, related);
    let included = included(&tx, related_to_jobs, &opts.include)
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
