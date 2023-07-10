use axum::{extract::State, http::StatusCode};
use eyre::Result;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, TransactionTrait};
use std::collections::HashMap;

use crate::{
    api::{
        documents::Meta,
        extract::{Json, Path},
        internal::documents::{ResourceType, TaskAttributes, TaskMeta, TaskRelation, TaskResource},
        jsonapi::{
            Document, DocumentData, Error, Related, Relation, Relationship, ResourceIdentifier,
        },
        AppState,
    },
    scheduling,
};

pub fn entity_to_resource(entity: &entity::Task) -> TaskResource {
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
                    id: entity.job,
                    meta: None,
                })),
            },
        )]
        .into(),
        meta: None,
    }
}
