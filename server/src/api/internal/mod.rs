mod downloads;
mod imports;

mod documents;

use axum::{middleware::from_fn, routing::get, Router};

use super::{auth, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/downloads", get(downloads::list))
        .route("/downloads/:id", get(downloads::list))
        .route("/imports", get(imports::imports).put(imports::begin))
        .route("/imports/:id", get(imports::import))
        // .route("/import/:job", patch(import::edit))
        // .route("/import/:job", post(import::run))
        // .route("/import/:job", delete(import::delete))
        .layer(from_fn(auth::auth_middleware))
}
