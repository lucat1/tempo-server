use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use jsonapi::model::JsonApiModel;
use sea_orm::EntityTrait;
use tower::util::ServiceExt;

use super::documents::Image;
use crate::response::{Error, Response};
use web::AppState;

fn image_to_image(image: &entity::Image) -> Image {
    Image {
        id: image.id.to_owned(),
        role: image.role.to_owned(),
        format: image.format.mime().to_string(),
        description: image.description.to_owned(),
        width: image.width,
        height: image.height,
        size: image.size,
    }
}

pub async fn image(
    State(AppState(db)): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, Error> {
    let image = entity::ImageEntity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not fetch image".to_string(),
                e.into(),
            )
        })?
        .ok_or(Error(
            StatusCode::NOT_FOUND,
            "Not found".to_string(),
            "Not found".into(),
        ))?;
    Ok(Response(image_to_image(&image).to_jsonapi_document()))
}

pub async fn file(
    State(AppState(db)): State<AppState>,
    Path(id): Path<String>,
    request: Request<Body>,
) -> Result<impl IntoResponse, Error> {
    let image = entity::ImageEntity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not fetch image".to_string(),
                e.into(),
            )
        })?
        .ok_or(Error(
            StatusCode::NOT_FOUND,
            "Not found".to_string(),
            "Not found".into(),
        ))?;
    Ok(
        tower_http::services::fs::ServeFile::new_with_mime(image.path, &image.format.mime())
            .oneshot(request)
            .await,
    )
}
