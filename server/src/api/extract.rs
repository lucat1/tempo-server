use super::jsonapi::Error;
use axum::{
    async_trait,
    body::{Bytes, HttpBody},
    extract::{
        rejection::TypedHeaderRejection, FromRequest, FromRequestParts, Json as AxumJson,
        Path as AxumPath, TypedHeader as AxumTypedHeader,
    },
    headers::{
        authorization::{Authorization, Bearer},
        Header, HeaderMap,
    },
    http::header::{self, HeaderValue},
    http::{request::Parts, Request, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use jsonwebtoken::{
    decode, errors::Error as JwtError, Algorithm, DecodingKey, TokenData, Validation,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use base::setting::{get_settings, SettingsError};

static HEADER_VALUE: &str = "application/vnd.api+json";

pub struct Json<T>(pub T);

#[derive(Debug, Error)]
pub enum JsonError {
    #[error("Invalid mime type, expected application/vnd.api+json")]
    Mime,
    #[error("Could not read body bytes: {}", .0)]
    BodyRead(#[from] axum::extract::rejection::BytesRejection),
    #[error("Invalid JSON structure: {}", .0)]
    Data(String),
    #[error("Invalid JSON syntax: {}", .0)]
    Syntax(String),
    #[error("IO error")]
    Io,
}

impl IntoResponse for JsonError {
    fn into_response(self) -> Response {
        let status = match self {
            JsonError::Io | JsonError::BodyRead(_) => StatusCode::INTERNAL_SERVER_ERROR,
            JsonError::Mime | JsonError::Syntax(_) | JsonError::Data(_) => StatusCode::BAD_REQUEST,
        };
        let err = Error {
            status,
            title: "Could not parse JSON request body".to_string(),
            detail: Some(self.into()),
        };
        (status, err).into_response()
    }
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for Json<T>
where
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
    T: for<'de> Deserialize<'de> + Send,
{
    type Rejection = JsonError;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        if jsonapi_content_type(req.headers()) {
            let bytes = Bytes::from_request(req, state).await?;
            let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);

            let value = match serde_path_to_error::deserialize(deserializer) {
                Ok(value) => value,
                Err(err) => {
                    let rejection = match err.inner().classify() {
                        serde_json::error::Category::Data => JsonError::Data(err.to_string()),
                        serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                            JsonError::Syntax(err.path().to_string())
                        }
                        serde_json::error::Category::Io => JsonError::Io,
                    };
                    return Err(rejection);
                }
            };

            Ok(Json(value))
        } else {
            Err(JsonError::Mime)
        }
    }
}

fn jsonapi_content_type(headers: &HeaderMap) -> bool {
    let content_type = if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        content_type
    } else {
        return false;
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return false;
    };

    mime.type_() == "application"
        && mime.subtype() == "vnd.api"
        && mime.suffix().map_or(false, |suffix| suffix == "json")
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (
            [(header::CONTENT_TYPE, HeaderValue::from_static(HEADER_VALUE))],
            AxumJson(self.0).into_response(),
        )
            .into_response()
    }
}

pub struct TypedHeader<T>(pub T);

#[derive(Debug, Error)]
pub enum TypedHeaderError {
    #[error("Could not get typed header")]
    TypedHeader(#[from] TypedHeaderRejection),
}

impl IntoResponse for TypedHeaderError {
    fn into_response(self) -> Response {
        Error {
            status: StatusCode::BAD_REQUEST,
            title: self.to_string(),
            detail: match self {
                TypedHeaderError::TypedHeader(e) => Some(Box::new(e)),
            },
        }
        .into_response()
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for TypedHeader<T>
where
    T: Header,
    S: Send + Sync,
{
    type Rejection = TypedHeaderError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AxumTypedHeader(t) = AxumTypedHeader::<T>::from_request_parts(parts, state).await?;
        Ok(Self(t))
    }
}

pub struct Path<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AxumPath(t) = AxumPath::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error {
                status: StatusCode::NOT_FOUND,
                title: "Invalid URL path".to_string(),
                detail: Some(e.into()),
            })?;
        Ok(Self(t))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClaimsSubject {
    Token,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
    pub sub: ClaimsSubject,
}

#[derive(Debug, Error)]
pub enum ClaimsError {
    #[error("Could not get the settings")]
    Settings(#[from] SettingsError),

    #[error("Could not get typed header")]
    TypedHeader(#[from] TypedHeaderError),

    #[error("Invalid authentication token")]
    Unauthorized(#[from] JwtError),
}

impl IntoResponse for ClaimsError {
    fn into_response(self) -> Response {
        Error {
            status: StatusCode::BAD_REQUEST,
            title: self.to_string(),
            detail: match self {
                ClaimsError::Settings(e) => Some(Box::new(e)),
                ClaimsError::TypedHeader(e) => Some(Box::new(e)),
                ClaimsError::Unauthorized(e) => Some(Box::new(e)),
            },
        }
        .into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = ClaimsError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(header) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
        check_token(header.token()).map(|td| td.claims)
    }
}

pub fn check_token<T>(token: &str) -> Result<TokenData<T>, ClaimsError>
where
    T: for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let settings = get_settings()?;
    let claims = decode::<T>(
        token,
        &DecodingKey::from_secret(settings.auth.jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    );
    match claims {
        Ok(token_data) => {
            tracing::trace!(?token_data, "User for request");
            Ok(token_data)
        }
        Err(e) => Err(ClaimsError::Unauthorized(e)),
    }
}
