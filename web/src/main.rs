mod aura;
mod internal;
mod response;

use axum::Router;
use clap::Parser;
use env_logger::{Builder, Env};
use eyre::{eyre, Result, WrapErr};
use log::info;
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use std::path::PathBuf;

use base::database::{get_database, open_database, DATABASE};
use base::setting::{load, SETTINGS};

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
    let env = Env::default()
        .filter_or(base::TAGGER_LOGLEVEL, "info")
        .write_style(base::TAGGER_STYLE);
    Builder::from_env(env)
        .filter(Some("sqlx"), log::LevelFilter::Warn)
        .init();

    let cli = Cli::parse();

    // settings
    SETTINGS.get_or_try_init(async { load(cli.config) }).await?;

    // database
    DATABASE
        .get_or_try_init(async { open_database().await })
        .await?;
    migration::Migrator::up(get_database()?, None).await?;

    let app = Router::new()
        .nest("/aura", aura::router())
        .nest("/internal", internal::router());

    let addr: SocketAddr = cli
        .listen_address
        .parse()
        .wrap_err(eyre!("Invalid listen address"))?;
    info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
