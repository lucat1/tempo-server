pub mod artists;
pub mod connections;
pub mod images;
pub mod mediums;
pub mod releases;
pub mod scrobbles;
pub mod search;
pub mod tracks;
pub mod users;

use axum::{middleware::from_fn, routing::get, Router};
use std::collections::HashMap;

use super::{
    auth,
    documents::{Included, ResourceType, ServerAttributes, ServerResource},
    extract::Json,
    jsonapi::{Document, DocumentData},
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
            get(users::relation)
                .post(users::post_relation)
                .delete(users::delete_relation),
        )
        .route("/connections", get(connections::connections))
        .route("/connections/:provider", get(connections::connection))
        .route("/search", get(search::search))
        .layer(from_fn(auth::auth_middleware))
        .route("/server", get(server))
        .route(
            "/auth",
            get(auth::auth).post(auth::login).patch(auth::refresh),
        )
        .route(
            "/connections/:provider/callback",
            get(connections::callback),
        )
}

async fn server() -> Json<Document<ServerResource, Included>> {
    Json(Document {
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
            meta: None,
        }),
        included: Vec::new(),
        links: HashMap::new(),
    })
}
