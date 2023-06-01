mod aura;
pub mod auth;
pub mod documents;
pub mod extract;
mod internal;
pub mod jsonapi;

use axum::Router;
use base::database::get_database;
use eyre::Result;
use sea_orm::DbConn;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState(pub DbConn);

pub fn router() -> Result<Router> {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);
    let tracing = TraceLayer::new_for_http();
    let conn = get_database()?.clone();
    Ok(Router::new()
        .nest("/aura", aura::router())
        .nest("/internal", internal::router())
        .layer(cors)
        .layer(tracing)
        .with_state(AppState(conn)))
}
