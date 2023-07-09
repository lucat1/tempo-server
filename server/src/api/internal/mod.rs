mod downloads;
mod import;
mod jobs;
mod tasks;

mod documents;

use axum::{
    middleware::from_fn,
    routing::{delete, get, patch, post, put},
    Router,
};

use super::{auth, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/jobs", get(jobs::list).put(jobs::schedule))
        .route("/downloads", get(downloads::list))
        .route("/downloads/:id", get(downloads::list))
        .route("/import", put(import::begin))
        .route("/import/:job", get(import::get))
        .route("/import/:job", patch(import::edit))
        .route("/import/:job", post(import::run))
        .route("/import/:job", delete(import::delete))
        // .route("/tasks/trigger/:type", post(tasks::trigger))
        .layer(from_fn(auth::auth_middleware))
}
