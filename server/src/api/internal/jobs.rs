use axum::{
    extract::{OriginalUri, State},
    http::StatusCode,
};
use eyre::Result;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, CursorTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::collections::HashMap;

use crate::{
    api::{
        documents::Meta,
        extract::{Json, Path},
        internal::documents::{
            InsertJobResource, JobAttributes, JobFilter, JobInclude, JobMeta, JobRelation,
            JobResource, ResourceType,
        },
        jsonapi::{
            make_cursor, Document, DocumentData, Error, InsertDocument, Query, Related, Relation,
            Relationship, ResourceIdentifier,
        },
        AppState,
    },
    scheduling,
};

use super::documents::TaskResource;

struct JobRelated {
    tasks: Vec<entity::Task>,
}

pub fn entity_to_resource(entity: &entity::Job, tasks: &[entity::Task]) -> JobResource {
    JobResource {
        id: entity.id,
        r#type: ResourceType::Job,
        attributes: JobAttributes {
            title: entity.title.to_owned(),
            description: entity.description.to_owned(),
            scheduled_at: entity.scheduled_at,
        },
        relationships: [(
            JobRelation::Task,
            Relationship {
                data: Relation::Multi(
                    tasks
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

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::Job>,
    light: bool,
) -> Result<Vec<JobRelated>, DbErr>
where
    C: ConnectionTrait,
{
    // TODO:
}

pub async fn schedule(
    State(AppState(db)): State<AppState>,
    json_jobs: Json<InsertDocument<InsertJobResource>>,
) -> Result<Json<Document<JobResource, TaskResource>>, Error> {
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
        let (job, ids) = scheduling::trigger_job(tx, &job.attributes.r#type)
            .await
            .map_err(|e| Error {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                title: "Could not schedule job".to_string(),
                detail: Some(e.into()),
            })?;
        let mut cond = Condition::any();
        for id in ids.into_iter() {
            cond = cond.add(entity::TaskColumn::Id.eq(id));
        }
        queued_jobs.push((job, tasks));
        let tasks = entity::TaskEntity::find()
            .filter(cond)
            .all(&db)
            .await
            .map_err(|e| Error {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                title: "Couldn't fetch the assiociated tasks".to_string(),
                detail: Some(e.into()),
            })?;
    }

    Ok(Json::new(Document {
        data: DocumentData::Multi(
            queued_jobs
                .iter()
                .map(|(job, tasks)| entity_to_resource(job, tasks))
                .collect(),
        ),
        included: queued_jobs
            .iter()
            .flat_map(|(job, tasks)| {
                tasks
                    .into_iter()
                    .map(|t| super::tasks::entity_to_resource(t, job))
                    .collect::<Vec<_>>()
            })
            .collect(),
        links: HashMap::new(),
    }))
}

pub async fn list(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<JobFilter, entity::JobColumn, JobInclude, i64>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<JobResource, TaskResource>>, Error> {
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
    let jobs = jobs_cursor.all(&db).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch jobs page".to_string(),
        detail: Some(e.into()),
    })?;
    Ok(())
}
