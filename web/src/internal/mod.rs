mod import;
mod library;
mod list;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

pub fn router() -> Router {
    Router::new()
        .route("/list", get(list::list))
        .route("/library", get(library::list))
        .route("/import", put(import::begin))
        .route("/import/:job", patch(import::edit))
        .route("/import/:job", post(import::run))
        .route("/import/:job", delete(import::delete))
}
