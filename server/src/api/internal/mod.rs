mod downloads;
mod imports;
// mod jobs;
// mod tasks;

mod documents;

use axum::{middleware::from_fn, routing::get, Router};

use super::{auth, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/downloads", get(downloads::list))
        .route("/downloads/:id", get(downloads::list))
        // .route("/jobs", get(jobs::jobs).put(jobs::schedule))
        // .route("/jobs/:id", get(jobs::job))
        // .route("/tasks", get(tasks::tasks))
        // .route("/tasks/:id", get(tasks::task))
        .route("/imports", get(imports::imports).put(imports::begin))
        .route("/imports/:job", get(imports::import))
        // .route("/import/:job", patch(import::edit))
        // .route("/import/:job", post(import::run))
        // .route("/import/:job", delete(import::delete))
        .layer(from_fn(auth::auth_middleware))
}
