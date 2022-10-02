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

use async_once_cell::OnceCell;
use clap::{arg, Command};
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
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
                .alias("ls")
                .about("Lists all the music being tracked")
                .arg(arg!(FORMAT: -f --format [FORMAT] "Format the required objects"))
                .arg(arg!(OBJECT: -o --object [OBJECT] "The type of object to list"))
                .arg(arg!(FILTER: [FILTER] ... "Filter the listing")),
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
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(util::path_to_str(
                    &SETTINGS.get().ok_or(eyre!("Could not obtain settings"))?.db,
                )?)
                .create_if_missing(true),
        )
        .await
        .map_err(|e| eyre!(e))
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    theme::init_logger();

    SETTINGS.get_or_try_init(async { settings::load() }).await?;
    let db = DB.get_or_try_init(db()).await?;
    sqlx::migrate!().run(db).await?;

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("list", sub_matches)) => {
            let filters = sub_matches
                .get_many::<String>("FILTER")
                .map(|i| i.into_iter().collect::<Vec<_>>())
                .unwrap_or_default();
            let format = sub_matches.get_one::<String>("FORMAT");
            let object = sub_matches.get_one::<String>("OBJECT");
            list::list(filters, format, object).await
        }
        Some(("update", sub_matches)) => {
            let filters = sub_matches
                .get_many::<String>("FILTER")
                .map(|i| i.into_iter().collect::<Vec<_>>())
                .unwrap_or_default();
            update::update(filters).await
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
