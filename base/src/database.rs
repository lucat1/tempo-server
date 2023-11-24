use crate::setting::SettingsError;

use super::setting::get_settings;
use async_once_cell::OnceCell;
use lazy_static::lazy_static;
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

lazy_static! {
    pub static ref DATABASE: Arc<OnceCell<DatabaseConnection>> = Arc::new(OnceCell::new());
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database global store is unitialized")]
    Uninitialized,

    #[error("Could not load settings: {0}")]
    Settings(#[from] SettingsError),

    #[error("Error while connecting to the database: {0}")]
    Database(#[from] DbErr),
}

pub fn get_database() -> Result<&'static DatabaseConnection, DatabaseError> {
    DATABASE.get().ok_or(DatabaseError::Uninitialized)
}

pub async fn open_database() -> Result<DatabaseConnection, DatabaseError> {
    let url = &get_settings()?.db;
    tracing::trace! {%url, "Connecting to database"};
    let mut opt = ConnectOptions::new(url.to_owned());
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true);
    // .sqlx_logging_level(log::LevelFilter::Info);
    Database::connect(opt)
        .await
        .map_err(DatabaseError::Database)
}
