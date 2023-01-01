#![feature(async_closure)]

mod fetch;
mod library;
mod models;
mod rank;
mod settings;
mod theme;
mod track;
mod util;

mod import;
mod list;
mod update;

// automatically generated sql migrations
mod generated;

use async_once_cell::OnceCell;
use clap::{arg, Command};
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use log::{error, info};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Sqlite};
use sqlx_migrate::Migrator;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use settings::Settings;

pub const CLI_NAME: &str = "tagger";
pub const VERSION: &str = "0.1.0";
pub const GITHUB: &str = "github.com/lucat1/tagger";

// logging constants
pub const TAGGER_LOGLEVEL: &str = "TAGGER_LOGLEVEL";
pub const TAGGER_STYLE: &str = "TAGGER_STYLE";

lazy_static! {
    pub static ref SETTINGS: Arc<OnceCell<Settings>> = Arc::new(OnceCell::new());
    pub static ref DB: Arc<OnceCell<SqlitePool>> = Arc::new(OnceCell::new());
}

fn cli() -> Command<'static> {
    Command::new(CLI_NAME)
        .about("Manage and tag your music collection")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("list")
                .alias("ls")
                .about("Lists all the music being tracked")
                .arg(arg!(FORMAT: -f --format [FORMAT] "Format the required objects"))
                .arg(arg!(OBJECT: -o --object [OBJECT] "The type of object to list"))
                .arg(arg!(FILTER: [FILTER] ... "Filter the listing")),
        )
        .subcommand(
            Command::new("config")
                .about("Print the current configuration in TOML")
        )
        .subcommand(
            Command::new("update")
                .alias("up")
                .alias("fix")
                .about("Applies the needed changes to all the out-of-date tags of all files being tracked")
                .arg_required_else_help(false)
                .arg(arg!(FILTER: [FILTER] ... "Filter the collection items to fix")),
        )
        .subcommand(
            Command::new("import")
                .about("Imports an album directory (recursively) into the library")
                .arg_required_else_help(true)
                .arg(arg!(PATH: <PATH> ... "Folder(s) to import as an album").value_parser(clap::value_parser!(PathBuf))),
        )
}

async fn db() -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(util::path_to_str(
                    &SETTINGS.get().ok_or(eyre!("Could not obtain settings"))?.db,
                )?)
                .create_if_missing(true)
                .log_slow_statements(log::LevelFilter::Trace, Duration::from_secs(10))
                .clone(),
        )
        .await
        .map_err(|e| eyre!(e))?;
    // sqlx::query("PRAGMA journal_mode=WAL")
    //     .execute(&pool)
    //     .await?;
    // sqlx::query("PRAGMA busy_timeout=60000")
    //     .execute(&pool)
    //     .await?;
    Ok(pool)
}

async fn migrate() -> Result<()> {
    let db = DB.get().ok_or(eyre!("Database connection not found"))?;
    let conn = db.acquire().await?;
    let mut migrator: Migrator<Sqlite> = Migrator::new(conn.detach());
    migrator.add_migrations(generated::migrations());

    migrator.verify().await?;
    let summary = migrator.migrate_all().await?;
    let diff = summary.new_version.unwrap_or_default() - summary.old_version.unwrap_or_default();
    if diff > 0 {
        info!(
            "Applied {} mirgation{}",
            diff,
            if diff > 1 { 's' } else { '\0' }
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    theme::init_logger();

    SETTINGS.get_or_try_init(async { settings::load() }).await?;

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("config", _)) => settings::print(),
        Some((a, b)) => {
            // all subcommands that require a database connection go in here
            DB.get_or_try_init(db()).await?;
            migrate().await?;

            match (a, b) {
                ("list", sub_matches) => {
                    let filters = sub_matches
                        .get_many::<String>("FILTER")
                        .map(|i| i.into_iter().collect::<Vec<_>>())
                        .unwrap_or_default();
                    let format = sub_matches.get_one::<String>("FORMAT");
                    let object = sub_matches.get_one::<String>("OBJECT");
                    list::list(filters, format, object).await
                }
                ("update", sub_matches) => {
                    let filters = sub_matches
                        .get_many::<String>("FILTER")
                        .map(|i| i.into_iter().collect::<Vec<_>>())
                        .unwrap_or_default();
                    update::update(filters).await
                }
                ("import", sub_matches) => {
                    let stream = sub_matches
                        .get_many::<PathBuf>("PATH")
                        .ok_or(eyre!("Expected at least one path argument to import"))?
                        .into_iter()
                        .collect::<Vec<_>>();
                    for p in stream.iter() {
                        import::import(p).await?;
                    }
                    Ok(())
                }
                (cmd, _) => {
                    error!(
                        "Invalid command {}, use `help` to see all available subcommands",
                        cmd
                    );
                    Ok(())
                }
            }
        }
        None => {
            error!(
                "A subcommand is required. Use `help` to get a list of all available subcommands"
            );
            Ok(())
        }
    }
}
