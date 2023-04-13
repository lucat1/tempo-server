mod import;
mod library;
mod list;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

pub fn router() -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    Router::new()
        .route("/list", get(list::list))
        .route("/library", get(library::list))
        .route("/import", put(import::begin))
        .route("/import/:job", get(import::get))
        .route("/import/:job", patch(import::edit))
        .route("/import/:job", post(import::run))
        .route("/import/:job", delete(import::delete))
        .layer(cors)
}
