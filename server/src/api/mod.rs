pub mod auth;
pub mod documents;
pub mod error;
pub mod extract;
mod internal;
pub mod jsonapi;
mod tempo;

use axum::{
    http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Router,
};
use base::database::get_database;
pub use error::Error;
use eyre::Result;
use sea_orm::DbConn;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState(pub DbConn);

pub fn router() -> Result<Router> {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(AllowOrigin::mirror_request())
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);
    let tracing = TraceLayer::new_for_http();
    let conn = get_database()?.clone();
    Ok(Router::new()
        .nest("/tempo", tempo::router())
        .nest("/internal", internal::router())
        .layer(cors)
        .layer(tracing)
        .with_state(AppState(conn)))
}
