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
        .route(
            "/imports/:id",
            get(imports::import)
                .patch(imports::edit)
                .post(imports::run)
                .delete(imports::delete),
        )
        .layer(from_fn(auth::auth_middleware))
}
