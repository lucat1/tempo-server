mod list;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new().route("/list", get(list::list))
}
