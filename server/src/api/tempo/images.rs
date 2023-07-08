use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use sea_orm::EntityTrait;
use tower::util::ServiceExt;

use crate::api::{
    documents::{ImageAttributes, ImageResource, Included, ResourceType},
    extract::Json,
    jsonapi::{Document, DocumentData, Error},
    AppState,
};

pub fn entity_to_resource(image: &entity::Image) -> ImageResource {
    ImageResource {
        id: image.id.to_owned(),
        r#type: ResourceType::Image,
        attributes: ImageAttributes {
            role: image.role.to_owned(),
            format: image.format.mime().to_string(),
            description: image.description.to_owned(),
            width: image.width,
            height: image.height,
            size: image.size,
        },
        meta: None,
        relationships: HashMap::new(),
    }
}

pub fn entity_to_included(image: &entity::Image) -> Included {
    Included::Image(entity_to_resource(image))
}

pub async fn image(
    State(AppState(db)): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Document<ImageResource, Included>>, Error> {
    let image = entity::ImageEntity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch image".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Image not found".to_string(),
            detail: Some("Not found".into()),
        })?;
    Ok(Json::new(Document {
        data: DocumentData::Single(entity_to_resource(&image)),
        included: Vec::new(),
        links: HashMap::new(),
    }))
}

pub async fn file(
    State(AppState(db)): State<AppState>,
    Path(id): Path<String>,
    request: Request<Body>,
) -> Result<impl IntoResponse, Error> {
    let image = entity::ImageEntity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch image".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Image file not found".to_string(),
            detail: Some("Not found".into()),
        })?;
    Ok(
        tower_http::services::fs::ServeFile::new_with_mime(image.path, &image.format.mime())
            .oneshot(request)
            .await,
    )
}
