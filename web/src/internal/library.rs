use axum::http::StatusCode;
use axum::Json;
use base::setting::{get_settings, Library};
use eyre::Result;
use log::trace;

pub async fn list() -> Result<Json<Vec<Library>>, StatusCode> {
    Ok(Json(
        get_settings()
            .map_err(|e| {
                trace!("Could not get settings: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .libraries
            .clone(),
    ))
}
