use axum::{routing::get, Router};
use jsonapi::api::*;
use jsonapi::jsonapi_model;
use jsonapi::model::*;
use serde::Deserialize;
use serde::Serialize;

use crate::response::Response;

pub fn router() -> Router {
    Router::new().route("/", get(root))
}

#[derive(Serialize, Deserialize)]
struct Server {
    id: String,
    #[serde(rename = "aura-version")]
    aura_version: String,
    server: String,
    #[serde(rename = "server-version")]
    server_version: String,
    #[serde(rename = "auth-required")]
    auth_required: bool,
    features: Vec<String>,
}

jsonapi_model!(Server; "server");

async fn root() -> Response {
    Response(
        Server {
            id: "0".to_string(),
            aura_version: "0.2.0".to_string(),
            server: shared::CLI_NAME.to_string(),
            server_version: shared::VERSION.to_string(),
            auth_required: false,
            features: vec!["albums".to_string(), "artists".to_string()],
        }
        .to_jsonapi_document(),
    )
}
