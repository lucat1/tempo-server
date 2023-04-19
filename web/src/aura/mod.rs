mod documents;
mod images;
mod tracks;

use axum::{routing::get, Router};
use jsonapi::api::*;
use jsonapi::jsonapi_model;
use jsonapi::model::*;
use serde::{Deserialize, Serialize};

use crate::response::Response;
use web::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/server", get(server))
        .route("/tracks", get(tracks::tracks))
        .route("/tracks/:id", get(tracks::track))
        .route("/tracks/:id/audio", get(tracks::audio))
        .route("/images/:id", get(images::image))
        .route("/images/:id/file", get(images::file))
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

async fn server() -> Response {
    Response(
        Server {
            id: "0".to_string(),
            aura_version: "0.2.0".to_string(),
            server: base::CLI_NAME.to_string(),
            server_version: base::VERSION.to_string(),
            auth_required: false,
            features: vec![],
            // TODO
            // features: vec!["albums".to_string(), "artists".to_string()],
        }
        .to_jsonapi_document(),
    )
}
