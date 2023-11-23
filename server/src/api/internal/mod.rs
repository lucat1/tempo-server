pub mod documents;
pub mod downloads;
pub mod imports;
pub mod update;

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};

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
        .route("/update/:update_type/all", post(update::all))
        .route("/update/:update_type/outdated", post(update::outdated))
        .layer(from_fn(auth::auth_middleware))
}
