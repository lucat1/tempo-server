use axum::http::StatusCode;
use axum::Json;
use base::setting::{get_settings, Library};
use eyre::Result;

pub async fn list() -> Result<Json<Vec<Library>>, StatusCode> {
    Ok(Json(
        get_settings()
            .map_err(|error| {
                tracing::warn! {%error, "Could not get settings"};
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .libraries
            .clone(),
    ))
}
