mod import;
mod list;
mod tasks;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

use web::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/list", get(list::list))
        .route("/import", put(import::begin))
        .route("/import/:job", get(import::get))
        .route("/import/:job", patch(import::edit))
        .route("/import/:job", post(import::run))
        .route("/import/:job", delete(import::delete))
        .route("/tasks/trigger/:type", post(tasks::trigger))
}
