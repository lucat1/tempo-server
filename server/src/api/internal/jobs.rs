use axum::{extract::State, http::StatusCode};
use eyre::Result;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, TransactionTrait};
use std::collections::HashMap;

use crate::{
    api::{
        documents::Meta,
        extract::{Json, Path},
        internal::documents::{
            InsertJobResource, JobAttributes, JobMeta, JobRelation, JobResource, ResourceType,
        },
        jsonapi::{
            Document, DocumentData, Error, InsertDocument, Related, Relation, Relationship,
            ResourceIdentifier,
        },
        AppState,
    },
    scheduling,
};

use super::documents::TaskResource;

pub async fn list() -> Result<(), StatusCode> {
    Ok(())
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
        let tasks = entity::TaskEntity::find()
            .filter(cond)
            .all(&db)
            .await
            .map_err(|e| Error {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                title: "Couldn't fetch the assiociated tasks".to_string(),
                detail: Some(e.into()),
            })?;
        queued_jobs.push((job, tasks));
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
