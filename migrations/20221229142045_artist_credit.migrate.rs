use crate::fetch::get;
use anyhow::Error;
use sqlx::{FromRow, Sqlite};
use sqlx_migrate::prelude::*;

#[derive(FromRow)]
pub struct Artist {
    pub mbid: Option<String>,
    pub name: String,
    pub join_phrase: Option<String>,
    pub sort_name: Option<String>,
    pub instruments: Vec<String>,
}

pub async fn artist_credit(mut ctx: MigrationContext<'_, Sqlite>) -> Result<(), MigrationError> {
    let (release, tracks) = get("test").await.map_err(Error::msg)?;
    println!("{:?} {:?}", release, tracks);

    Ok(())
}
