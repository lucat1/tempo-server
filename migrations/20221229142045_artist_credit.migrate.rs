use crate::fetch::{get, CLIENT, MB_USER_AGENT};
use reqwest::header::USER_AGENT;
use sqlx::{Executor, FromRow, Sqlite};
use sqlx_migrate::prelude::*;

#[derive(FromRow)]
pub struct Artist {
    pub mbid: Option<String>,
    pub name: String,
    pub join_phrase: Option<String>,
    pub sort_name: Option<String>,
    pub instruments: Vec<String>,
}

pub async fn migrate_artist_group(
    mut ctx: MigrationContext<'_, Sqlite>,
) -> Result<(), MigrationError> {
    let copy_client = CLIENT.clone();

    let limited_client = tower::ServiceBuilder::new()
        .rate_limit(10, Duration::from_secs())
        .service(CLIENT);

    CLIENT = limited_client;

    let (release, tracks) = get("test").await?;
    println!("{:?} {:?}", release, tracks);

    CLIENT = copy_client;
    Ok(())
}
