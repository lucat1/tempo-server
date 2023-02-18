use clap::{arg, Command};
use env_logger::{Builder, Env};
use eyre::Result;
use sea_orm_migration::MigratorTrait;

use shared::database::{get_database, open_database, DATABASE};
use shared::setting::{load, SETTINGS};

pub const CLI_NAME: &str = "tagger";
pub const VERSION: &str = "0.1.0";
pub const GITHUB: &str = "github.com/lucat1/tagger";

// logging constants
pub const TAGGER_LOGLEVEL: &str = "TAGGER_LOGLEVEL";
pub const TAGGER_STYLE: &str = "TAGGER_STYLE";

fn cli() -> Command<'static> {
    Command::new(CLI_NAME)
        .about("Manage and explore you music collection")
        .arg(arg!(CONFIG: -c --config [CONFIG] "The path for the config"))
}

#[tokio::main]
async fn main() -> Result<()> {
    // logging
    color_eyre::install()?;
    let env = Env::default()
        .filter_or(TAGGER_LOGLEVEL, "info")
        .write_style(TAGGER_STYLE);
    Builder::from_env(env)
        .filter(Some("sqlx"), log::LevelFilter::Warn)
        .init();

    // settings
    SETTINGS.get_or_try_init(async { load() }).await?;

    // database
    DATABASE
        .get_or_try_init(async { open_database().await })
        .await?;
    migration::Migrator::up(get_database()?, None).await?;

    let matches = cli().get_matches();
    println!("cli matches {:?}", matches);
    Ok(())
}
