mod fetch;
mod internal;
mod rank;
mod theme;
mod track;

mod import;
mod list;
mod update;

use clap::{arg, Command};
use eyre::{eyre, Result};
use log::error;
use sea_orm_migration::MigratorTrait;
use std::path::PathBuf;

use shared::database::{get_database, open_database, DATABASE};
use shared::setting::{load, print, SETTINGS};

pub const CLI_NAME: &str = "tagger";
pub const VERSION: &str = "0.1.0";
pub const GITHUB: &str = "github.com/lucat1/tagger";

// logging constants
pub const TAGGER_LOGLEVEL: &str = "TAGGER_LOGLEVEL";
pub const TAGGER_STYLE: &str = "TAGGER_STYLE";

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

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    theme::init_logger();

    SETTINGS.get_or_try_init(async { load() }).await?;

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("config", _)) => print(),
        Some((a, b)) => {
            // all subcommands that require a database connection go in here
            DATABASE
                .get_or_try_init(async { open_database().await })
                .await?;
            migration::Migrator::up(get_database()?, None).await?;

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
