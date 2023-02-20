use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use bytes::{BufMut, BytesMut};
use jsonapi::api::*;
use serde::Serialize;

fn json_to_response(data: impl Serialize) -> bytes::Bytes {
    // Use a small initial capacity of 128 bytes like serde_json::to_vec
    // https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
    let mut buf = BytesMut::with_capacity(128).writer();
    match serde_json::to_writer(&mut buf, &data) {
        Ok(()) => buf.into_inner().freeze(),
        Err(err) => err.to_string().into(),
    }
}

pub struct Response(pub JsonApiDocument);

impl IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        match self.0 {
            JsonApiDocument::Error(err) => (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/vnd.api+json")],
                json_to_response(err),
            )
                .into_response(),
            JsonApiDocument::Data(data) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/vnd.api+json")],
                json_to_response(data),
            )
                .into_response(),
        }
    }
}
