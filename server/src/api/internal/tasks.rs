use axum::{extract::Path, http::StatusCode};
use base::setting::TaskType;
use eyre::Result;

use crate::scheduling;

// pub async fn list() -> Result<(), StatusCode> {}

pub async fn trigger(Path(task): Path<TaskType>) -> Result<(), StatusCode> {
    scheduling::trigger_task(&task).await.map_err(|error| {
        tracing::warn!(%error, ?task, "Error while trigger task");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
