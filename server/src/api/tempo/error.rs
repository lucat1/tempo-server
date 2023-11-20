use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use thiserror::Error;

use crate::api::jsonapi::Error;

#[derive(Error, Debug)]
pub enum TempoError {
    #[error("Database error")]
    DbErr(#[from] DbErr),

    #[error("Not found")]
    NotFound(Option<DbErr>),
}

impl TempoError {
    fn status(&self) -> StatusCode {
        match self {
            TempoError::DbErr(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TempoError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

impl From<TempoError> for Error {
    fn from(value: TempoError) -> Self {
        Self {
            status: value.status(),
            title: value.to_string(),
            detail: match value {
                TempoError::DbErr(e) => Some(e.into()),
                TempoError::NotFound(o) => o.map(|e| e.into()),
            },
        }
    }
}

impl IntoResponse for TempoError {
    fn into_response(self) -> Response {
        <Self as Into<Error>>::into(self).into_response()
    }
}
