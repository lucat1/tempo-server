mod tag;
mod import;
mod util;

use std::path::PathBuf;
use eyre::{eyre, Result};
use clap::{arg, Command};

static CLI_NAME: &str = "tagger";

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

fn main() -> Result<()> {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("list", sub_matches)) => {
            let filter = sub_matches.get_one::<String>("Filter").ok_or(eyre!("Filter argument expected"))?;
                println!("list: {}", filter);
                Ok(())
        }
        Some(("fix", sub_matches)) => {
            let filter = sub_matches.get_one::<String>("Filter").ok_or(eyre!("Filter argument expected"))?;
                println!("fix: {}", filter);
                Ok(())
        }
        Some(("import", sub_matches)) => {
            sub_matches
                .get_many::<PathBuf>("PATH")
                .ok_or(eyre!("Expected at least one path argument to import"))?
                .into_iter()
                .map(|p| import::import(p))
                .collect()
        }
        _ => unreachable!(),
    }
}
