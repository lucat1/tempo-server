use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use thiserror::Error;

use crate::{
    api::{
        internal::update::UpdateError, jsonapi::Error as JsonAPIError,
        tempo::connections::ConnectionError,
    },
    search::SearchError,
};
use base::setting::SettingsError;

use super::{auth::AuthError, extract::ClaimsError};

#[derive(Error, Debug)]
pub enum Error {
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
    #[error("Could not authenticate: {0}")]
    Auth(#[from] AuthError),
    #[error("Could not fetch auth claims: {0}")]
    Claims(#[from] ClaimsError),

    #[error("Could not run update: {0}")]
    Update(#[from] UpdateError),

    #[error("Track does not have an associated path")]
    NoTrackPath,
    #[error("Track does not have an associated format")]
    NoTrackFormat,
}

impl Error {
    fn status(&self) -> StatusCode {
        match self {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::NotModified => StatusCode::NOT_MODIFIED,
            Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<Error> for JsonAPIError {
    fn from(value: Error) -> Self {
        Self {
            status: value.status(),
            title: value.to_string(),
            detail: match value {
                Error::DbErr(e) => Some(e.into()),
                Error::NotFound(o) => o.map(|e| e.into()),
                Error::Unauthorized(Some(v)) => Some(v.into()),
                Error::BadRequest(Some(v)) => Some(v.into()),
                _ => None,
            },
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        <Self as Into<JsonAPIError>>::into(self).into_response()
    }
}
