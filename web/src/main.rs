mod aura;
pub mod auth;
pub mod documents;
pub mod fetch;
mod internal;
pub mod jsonapi;
mod scheduling;
pub mod search;
pub mod tasks;

use axum::Router;
use clap::Parser;
use eyre::{eyre, Result, WrapErr};
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
    prelude::*,
};

use crate::search::{open_index_writers, open_indexes, INDEXES, INDEX_WRITERS};
use base::setting::{load, SETTINGS};
use base::{
    database::{get_database, open_database, DATABASE},
    setting::get_settings,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, name = "ADDRESS", default_value_t = String::from("127.0.0.1:3000"))]
    listen_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // logging
    color_eyre::install()?;

    let subscriber = tracing_subscriber::registry().with(fmt::layer()).with(
        EnvFilter::builder()
            .with_default_directive(LevelFilter::TRACE.into())
            .with_env_var(base::TEMPO_LOGLEVEL)
            .from_env_lossy(),
    );
    tracing::subscriber::set_global_default(subscriber)?;

    let cli = Cli::parse();

    // settings
    SETTINGS.get_or_try_init(async { load(cli.config) }).await?;

    // database
    DATABASE
        .get_or_try_init(async { open_database().await })
        .await?;
    migration::Migrator::up(get_database()?, None).await?;

    // search index
    INDEXES.get_or_try_init(async { open_indexes() }).await?;
    INDEX_WRITERS
        .lock()
        .await
        .get_or_try_init(async { open_index_writers() })
        .await?;

    // background tasks
    crate::tasks::queue_loop()?;
    let mut scheduler = scheduling::new().await?;
    for (task, schedule) in get_settings()?.tasks.recurring.iter() {
        scheduling::schedule(&mut scheduler, schedule.to_owned(), task.to_owned()).await?;
    }
    scheduling::start(&mut scheduler).await?;

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);
    let tracing = TraceLayer::new_for_http();
    let conn = get_database()?.clone();
    let app = Router::new()
        .nest("/aura", aura::router())
        .nest("/internal", internal::router())
        .layer(cors)
        .layer(tracing)
        .with_state(jsonapi::AppState(conn));

    let addr: SocketAddr = cli
        .listen_address
        .parse()
        .wrap_err(eyre!("Invalid listen address"))?;
    tracing::info! {%addr, "Listening"};
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
