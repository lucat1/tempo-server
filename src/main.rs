mod fetch;
mod import;
mod library;
mod models;
mod rank;
mod settings;
mod theme;
mod track;
mod util;

use async_once_cell::OnceCell;
use clap::{arg, Command};
use directories::ProjectDirs;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

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
                .about("Lists all the music being tracked")
                .arg(arg!(<FILTER> "Filter the listing"))
                .arg_required_else_help(false),
        )
        .subcommand(
            Command::new("fix")
                .about("Applies the needed changes to all the out-of-date tags of all files being tracked")
                .arg(arg!(<FILTER> "Filter the part of your collection to fix"))
                .arg_required_else_help(false),
        )
        .subcommand(
            Command::new("import")
                .about("Imports an album directory (recursively) into the library")
                .arg_required_else_help(true)
                .arg(arg!(<PATH> ... "Folder(s) to import as an album").value_parser(clap::value_parser!(PathBuf))),
        )
}

fn cfg() -> Result<Settings> {
    let dirs = ProjectDirs::from("com", "github", CLI_NAME)
        .ok_or(eyre!("Could not locate program directories"))?;
    let path = dirs.config_dir().join(PathBuf::from("config.toml"));
    match fs::read_to_string(path) {
        Ok(str) => toml::from_str(str.as_str()).map_err(|e| eyre!(e)),
        Err(_) => Settings::gen_default(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    theme::init_logger();

    SETTINGS.get_or_try_init(async { cfg() }).await?;
    let db = DB
        .get_or_try_init(async {
            SqlitePool::connect_with(
                SqliteConnectOptions::new()
                    .filename(util::path_to_str(
                        &SETTINGS.get().ok_or(eyre!("Could not obtain settings"))?.db,
                    )?)
                    .create_if_missing(true),
            )
            .await
            .map_err(|e| eyre!(e))
        })
        .await?;
    sqlx::migrate!().run(db).await?;

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("list", sub_matches)) => {
            let filter = sub_matches
                .get_one::<String>("Filter")
                .ok_or(eyre!("Filter argument expected"))?;
            Ok(())
        }
        Some(("fix", sub_matches)) => {
            let filter = sub_matches
                .get_one::<String>("Filter")
                .ok_or(eyre!("Filter argument expected"))?;
            Ok(())
        }
        Some(("import", sub_matches)) => {
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
        _ => unreachable!(),
    }
}
