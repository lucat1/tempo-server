use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::EntityTrait;
use std::collections::HashMap;
use uuid::Uuid;

use super::AppState;
use crate::documents::ArtistAttributes;
use crate::jsonapi::{ArtistResource, Document, DocumentData, Error, ResourceType};

pub fn entity_to_resource(entity: entity::Artist) -> ArtistResource {
    ArtistResource {
        r#type: ResourceType::Artist,
        id: entity.id,
        attributes: ArtistAttributes {
            name: entity.name,
            sort_name: entity.sort_name,
        },
        relationships: HashMap::new(),
    }
}

pub async fn artists(
    State(AppState(db)): State<AppState>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let artists = entity::ArtistEntity::find()
        .all(&db)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch all artists".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json(Document {
        data: DocumentData::Multi(artists.into_iter().map(entity_to_resource).collect()),
        included: Vec::new(),
    }))
}

pub async fn artist(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let artist = entity::ArtistEntity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried artist".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Artist not found".to_string(),
            detail: None,
        })?;
    Ok(Json(Document {
        data: DocumentData::Single(entity_to_resource(artist)),
        included: Vec::new(),
    }))
}
