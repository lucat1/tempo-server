use super::setting::get_settings;
use async_once_cell::OnceCell;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use log::trace;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::Arc;
use std::time::Duration;

lazy_static! {
    pub static ref DATABASE: Arc<OnceCell<DatabaseConnection>> = Arc::new(OnceCell::new());
}

pub fn get_database() -> Result<&'static DatabaseConnection> {
    DATABASE.get().ok_or(eyre!("Could not get the database"))
}

pub async fn open_database() -> Result<DatabaseConnection> {
    let url = format!("sqlite://{}?mode=rwc", get_settings()?.db.to_owned());
    trace!("Connecting to {}", url);
    let mut opt = ConnectOptions::new(url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Info);
    Database::connect(opt).await.map_err(|e| eyre!(e))
}
