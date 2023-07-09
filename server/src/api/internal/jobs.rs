use axum::{extract::State, http::StatusCode};
use eyre::Result;
use sea_orm::TransactionTrait;
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

pub async fn list() -> Result<(), StatusCode> {
    Ok(())
}

pub fn entity_to_resource(entity: &entity::Job, tasks: &[i64]) -> JobResource {
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
                                id: *t,
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
) -> Result<Json<Document<JobResource, ()>>, Error> {
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
        queued_jobs.push(
            scheduling::trigger_job(tx, &job.attributes.r#type)
                .await
                .map_err(|e| Error {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    title: "Could not schedule job".to_string(),
                    detail: Some(e.into()),
                })?,
        );
    }

    Ok(Json::new(Document {
        data: DocumentData::Multi(
            queued_jobs
                .into_iter()
                .map(|(job, ids)| entity_to_resource(&job, &ids))
                .collect(),
        ),
        included: Vec::new(),
        links: HashMap::new(),
    }))
}
