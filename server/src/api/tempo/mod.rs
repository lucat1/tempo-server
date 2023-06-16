mod artists;
mod connections;
mod images;
mod mediums;
mod releases;
mod scrobbles;
mod search;
mod tracks;
mod users;

use axum::{middleware::from_fn, routing::get, Router};
use std::collections::HashMap;

use super::{
    auth,
    documents::ServerAttributes,
    extract::Json,
    jsonapi::{Document, DocumentData, ResourceType, ServerResource},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
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
        .route(
            "/scrobbles",
            get(scrobbles::scrobbles).put(scrobbles::insert_scrobbles),
        )
        .route("/scrobbles/:id", get(scrobbles::scrobble))
        .route("/users/:username", get(users::user))
        .route(
            "/users/:username/relationships/:relation",
            get(users::relation),
        )
        .route("/connections", get(connections::providers))
        .route("/connections/:provider", get(connections::provider))
        .route("/search", get(search::search))
        .layer(from_fn(auth::auth_middleware))
        .route("/server", get(server))
        .route("/auth", get(auth::auth).post(auth::login))
        .route(
            "/connections/:provider/callback",
            get(connections::callback),
        )
}

async fn server() -> Json<Document<ServerResource>> {
    Json::new(Document {
        data: DocumentData::Single(ServerResource {
            r#type: ResourceType::Server,
            id: "0".to_string(),
            attributes: ServerAttributes {
                tempo_version: "0.1.0".to_string(),
                server: base::CLI_NAME.to_string(),
                server_version: base::VERSION.to_string(),
                auth_required: true,
                features: [
                    "artists",
                    "releases",
                    "mediums",
                    "tracks",
                    "users",
                    "scrobbles",
                    "connections",
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            },
            relationships: HashMap::new(),
            meta: HashMap::new(),
        }),
        included: Vec::new(),
        links: HashMap::new(),
    })
}
