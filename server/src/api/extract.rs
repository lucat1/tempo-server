use super::jsonapi::Error;
use axum::{
    async_trait,
    body::HttpBody,
    extract::{FromRequest, FromRequestParts, Json as AxumJson, TypedHeader as AxumTypedHeader},
    headers::Header,
    http::{request::Parts, Request, StatusCode},
    response::{IntoResponse, Response},
    BoxError,
};
use serde::{Deserialize, Serialize};

pub struct Json<T>(AxumJson<T>);

impl<T> Json<T> {
    pub fn new(value: T) -> Self {
        Self(AxumJson(value))
    }
    pub fn inner(self) -> T {
        self.0 .0
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
    type Rejection = Error;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        AxumJson::<T>::from_request(req, state)
            .await
            .map(|val| Json(val))
            .map_err(|e| {
                tracing::trace!(error = ?e, "Invalid JSON request");
                Error {
                    status: StatusCode::BAD_REQUEST,
                    title: "Invalid JSON input".to_string(),
                    detail: Some(e.into()),
                }
            })
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

pub struct TypedHeader<T>(AxumTypedHeader<T>);

impl<T> TypedHeader<T> {
    pub fn inner(self) -> T {
        self.0 .0
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for TypedHeader<T>
where
    T: Header,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        AxumTypedHeader::<T>::from_request_parts(parts, state)
            .await
            .map(|val| TypedHeader(val))
            .map_err(|e| Error {
                status: StatusCode::BAD_REQUEST,
                title: format!("Invalid header: {}", T::name()),
                detail: Some(e.into()),
            })
    }
}
