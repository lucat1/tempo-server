use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;
use uuid::Uuid;

use super::AppState;
use crate::documents::ArtistAttributes;
use crate::jsonapi::{ArtistResource, Document, DocumentData, Error, Query, ResourceType};

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
    Query(opts): Query<entity::ArtistColumn>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let mut artists_query = entity::ArtistEntity::find();
    for (sort_key, sort_order) in opts.sort.into_iter() {
        artists_query = artists_query.order_by(sort_key, sort_order);
    }
    for (filter_key, filter_value) in opts.filter.into_iter() {
        artists_query = artists_query.filter(ColumnTrait::eq(&filter_key, filter_value));
    }
    let artists = artists_query.all(&db).await.map_err(|e| Error {
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
