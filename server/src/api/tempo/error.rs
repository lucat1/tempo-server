use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use thiserror::Error;

use crate::{
    api::{jsonapi::Error, tempo::connections::ConnectionError},
    search::SearchError,
};
use base::setting::SettingsError;

#[derive(Error, Debug)]
pub enum TempoError {
    #[error("Database error")]
    DbErr(#[from] DbErr),

    #[error("Not found")]
    NotFound(Option<DbErr>),
    #[error("Not modified")]
    NotModified,
    #[error("Unauthorized")]
    Unauthorized(Option<String>),
    #[error("Bad request")]
    BadRequest(Option<String>),

    #[error("Could not read settings: {0}")]
    Settings(#[from] SettingsError),

    #[error("Could not operate on the connection: {0}")]
    Connection(#[from] ConnectionError),

    #[error("Could not operate on the search index: {0}")]
    Search(#[from] SearchError),

    #[error("Track does not have an associated path")]
    NoTrackPath,
    #[error("Track does not have an associated format")]
    NoTrackFormat,
}

impl TempoError {
    fn status(&self) -> StatusCode {
        match self {
            TempoError::NotFound(_) => StatusCode::NOT_FOUND,
            TempoError::NotModified => StatusCode::NOT_MODIFIED,
            TempoError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            TempoError::BadRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
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
                TempoError::Unauthorized(Some(v)) => Some(v.into()),
                TempoError::BadRequest(Some(v)) => Some(v.into()),
                _ => None,
            },
        }
    }
}

impl IntoResponse for TempoError {
    fn into_response(self) -> Response {
        <Self as Into<Error>>::into(self).into_response()
    }
}
