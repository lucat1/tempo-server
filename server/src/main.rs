mod api;
pub mod fetch;
pub mod import;
pub mod scheduling;
pub mod search;
pub mod tasks;

use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use rand::distributions::{Alphanumeric, DistString};
use sea_orm_migration::MigratorTrait;
use std::path::PathBuf;
use std::{fmt::Display, net::SocketAddr, str::FromStr};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
    prelude::*,
};

use argon2::Argon2;
use password_hash::{PasswordHasher, SaltString};
use pbkdf2::Pbkdf2;
use scrypt::Scrypt;

use crate::search::{open_index_writers, open_indexes, INDEXES, INDEX_WRITERS};
use base::setting::{load, Settings, SETTINGS};
use base::{
    database::{get_database, open_database, DATABASE},
    setting::{generate_default, get_settings},
    CLI_NAME,
};
use tasks::{open_taskie_client, TASKIE_CLIENT};

#[derive(Parser)]
#[command(name = CLI_NAME,author, version, about, long_about = None)]
#[command(next_line_help = true)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, name = "ADDRESS", default_value_t = String::from("127.0.0.1:4000"))]
    listen_address: String,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    DefaultConfig,
    HashPassword(HashPasswordOptions),
    Serve,
}

#[derive(Parser)]
struct HashPasswordOptions {
    #[arg(short, long, name = "ALGORITHM", default_value_t = HashPasswordAlgorithm::Argon2)]
    algorithm: HashPasswordAlgorithm,

    #[arg(short, long, name = "SALT")]
    salt: Option<String>,

    #[arg(name = "PASSWORD", help = "The password you want to hash")]
    password: String,
}

#[derive(Clone)]
enum HashPasswordAlgorithm {
    Argon2,
    Pbkdf2,
    Scrypt,
}

impl Display for HashPasswordAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashPasswordAlgorithm::Argon2 => write!(f, "argon2"),
            HashPasswordAlgorithm::Pbkdf2 => write!(f, "pbkdf2"),
            HashPasswordAlgorithm::Scrypt => write!(f, "scrypt"),
        }
    }
}

impl FromStr for HashPasswordAlgorithm {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "argon2" => Ok(HashPasswordAlgorithm::Argon2),
            "pbkdf2" => Ok(HashPasswordAlgorithm::Pbkdf2),
            "scrypt" => Ok(HashPasswordAlgorithm::Scrypt),
            s => Err(format!("Invalid hashing algorithm: {}", s)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // logging
    color_eyre::install()?;
    let tracing_builder = tracing_subscriber::registry().with(fmt::layer());
    if std::env::var(base::TEMPO_LOGLEVEL).is_ok() {
        tracing_builder.with(EnvFilter::from_env(base::TEMPO_LOGLEVEL))
    } else {
        tracing_builder.with(EnvFilter::default().add_directive(LevelFilter::INFO.into()))
    }
    .init();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::DefaultConfig => {
            let mut default = Settings::default();
            default = generate_default(default)?;
            let str = toml::to_string(&default)?;
            println!("{}", str);
            Ok(())
        }
        Command::HashPassword(opts) => {
            let salt = opts
                .salt
                .unwrap_or_else(|| Alphanumeric.sample_string(&mut rand::thread_rng(), 32));
            let salt_str = SaltString::from_b64(salt.as_str())?;
            let hash = match opts.algorithm {
                HashPasswordAlgorithm::Argon2 => {
                    Argon2::default().hash_password(opts.password.as_bytes(), &salt_str)
                }
                HashPasswordAlgorithm::Pbkdf2 => {
                    Pbkdf2.hash_password(opts.password.as_bytes(), &salt_str)
                }
                HashPasswordAlgorithm::Scrypt => {
                    Scrypt.hash_password(opts.password.as_bytes(), &salt_str)
                }
            }?;
            println!("{}", hash);
            Ok(())
        }
        Command::Serve => {
            // settings
            SETTINGS.get_or_try_init(async { load(cli.config) }).await?;
            TASKIE_CLIENT
                .get_or_try_init(async { open_taskie_client().await })
                .await?;

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

            let addr: SocketAddr = cli
                .listen_address
                .parse()
                .wrap_err(eyre!("Invalid listen address"))?;
            tracing::info! {%addr, "Listening"};
            let router = api::router()?;
            axum::Server::bind(&addr)
                .serve(router.into_make_service())
                .await
                .unwrap();
            Ok(())
        }
    }
}
