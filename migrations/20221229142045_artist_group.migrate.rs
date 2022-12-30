use crate::fetch::{get, CLIENT};
use sqlx::{FromRow, Sqlite};
use sqlx_migrate::prelude::*;
use std::time::Duration;
use tower::{limit::rate::RateLimit, ServiceBuilder};

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

    let limited_client = ServiceBuilder::new()
        .rate_limit(300, Duration::from_secs(1))
        .service(CLIENT);

    CLIENT = limited_client;

    let (release, tracks) = get("test").await?;
    println!("{:?} {:?}", release, tracks);

    CLIENT = copy_client;
    Ok(())
}
