use axum::{
    extract::{OriginalUri, State},
    http::StatusCode,
};
use eyre::Result;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, CursorTrait, DbErr, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use std::collections::HashMap;

use super::jobs;
use crate::api::{
    extract::{Json, Path},
    internal::documents::{
        dedup, Included, ResourceType, TaskAttributes, TaskFilter, TaskInclude, TaskRelation,
        TaskResource,
    },
    jsonapi::{
        links_from_resource, make_cursor, Document, DocumentData, Error, Query, Related, Relation,
        Relationship, ResourceIdentifier,
    },
    AppState,
};

#[derive(Default)]
pub struct TaskRelated {
    job: i64,
}

pub fn entity_to_resource(entity: &entity::Task, related: &TaskRelated) -> TaskResource {
    TaskResource {
        id: entity.id,
        r#type: ResourceType::Task,
        attributes: TaskAttributes {
            data: entity.data.to_owned(),
            description: entity.description.to_owned(),

            scheduled_at: entity.scheduled_at,
            started_at: entity.started_at,
            ended_at: entity.ended_at,
        },
        relationships: [(
            TaskRelation::Job,
            Relationship {
                data: Relation::Single(Related::Int(ResourceIdentifier {
                    r#type: ResourceType::Job,
                    id: related.job,
                    meta: None,
                })),
            },
        )]
        .into(),
        meta: None,
    }
}
pub fn entity_to_included(entity: &entity::Task, related: &TaskRelated) -> Included {
    Included::Task(entity_to_resource(entity, related))
}

pub async fn related<C>(
    _db: &C,
    entities: &[entity::Task],
    _light: bool,
) -> Result<Vec<TaskRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let mut result = Vec::new();
    for task in entities.iter() {
        result.push(TaskRelated { job: task.job });
    }
    Ok(result)
}

pub async fn included<C>(
    db: &C,
    related: Vec<TaskRelated>,
    include: &[TaskInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&TaskInclude::Job) {
        let mut cond = Condition::any();
        for rel in related.iter() {
            cond = cond.add(ColumnTrait::eq(&entity::JobColumn::Id, rel.job));
        }
        let jobs = entity::JobEntity::find().filter(cond).all(db).await?;
        let jobs_related = jobs::related(db, &jobs, true).await?;
        for (i, job) in jobs.iter().enumerate() {
            included.push(jobs::entity_to_included(job, &jobs_related[i]))
        }
    }
    Ok(included)
}

pub async fn tasks(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<TaskFilter, entity::TaskColumn, TaskInclude, i64>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<TaskResource, Included>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let mut tasks_query = entity::TaskEntity::find();
    for (sort_key, sort_order) in opts.sort.iter() {
        tasks_query = tasks_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    let mut _tasks_cursor = tasks_query.cursor_by(entity::ArtistColumn::Id);
    let tasks_cursor = make_cursor(&mut _tasks_cursor, &opts.page);
    let tasks = tasks_cursor.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch tasks page".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_tasks = related(&db, &tasks, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch the related entites".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, task) in tasks.iter().enumerate() {
        data.push(entity_to_resource(task, &related_to_tasks[i]));
    }
    let included = included(&tx, related_to_tasks, &opts.include)
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

pub async fn task(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<TaskFilter, entity::TaskColumn, TaskInclude, i64>,
    task_path: Path<i64>,
) -> Result<Json<Document<TaskResource, Included>>, Error> {
    let id = task_path.inner();
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let task = entity::TaskEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried task".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Task not found".to_string(),
            detail: None,
        })?;
    let related_to_tasks = related(&tx, &[task.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the related tasks".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = TaskRelated::default();
    let related = related_to_tasks.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&task, related);
    let included = included(&tx, related_to_tasks, &opts.include)
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
