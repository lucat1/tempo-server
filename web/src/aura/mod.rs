mod artists;
mod images;
mod mediums;
mod releases;
mod tracks;

use std::collections::HashMap;

use axum::{routing::get, Json, Router};

use crate::{
    documents::ServerAttributes,
    jsonapi::{Document, DocumentData, ResourceType, ServerResource},
};

use web::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/server", get(server))
        .route("/images/:id", get(images::image))
        .route("/images/:id/file", get(images::file))
        .route("/artists", get(artists::artists))
        .route("/artists/:id", get(artists::artist))
        .route("/releases", get(releases::releases))
        .route("/releases/:id", get(releases::release))
        .route("/mediums/", get(mediums::mediums))
        .route("/mediums/:id", get(mediums::medium))
        .route("/tracks", get(tracks::tracks))
        .route("/tracks/:id", get(tracks::track))
        .route("/tracks/:id/audio", get(tracks::audio))
}

async fn server() -> Json<Document<ServerResource>> {
    Json(Document {
        data: DocumentData::Single(ServerResource {
            r#type: ResourceType::Server,
            id: "0".to_string(),
            attributes: ServerAttributes {
                aura_version: "0.2.0".to_string(),
                server: base::CLI_NAME.to_string(),
                server_version: base::VERSION.to_string(),
                auth_required: false,
                features: ["artists", "releases", "mediums", "tracks"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            relationships: HashMap::new(),
        }),
        included: Vec::new(),
    })
}
