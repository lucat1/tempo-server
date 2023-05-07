use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use sea_orm::EntityTrait;
use tower::util::ServiceExt;

use crate::documents::ImageAttributes;
use crate::jsonapi::{Document, DocumentData, Error, ImageResource, Included, ResourceType};
use web::AppState;

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
        relationships: HashMap::new(),
    }
}

pub fn entity_to_included(image: &entity::Image) -> Included {
    Included::Image(entity_to_resource(image))
}

pub async fn image(
    State(AppState(db)): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Document<ImageResource>>, Error> {
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
    Ok(Json(Document {
        data: DocumentData::Single(entity_to_resource(&image)),
        included: Vec::new(),
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
