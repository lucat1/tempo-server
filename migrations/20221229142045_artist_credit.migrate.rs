use crate::fetch::get;
use anyhow::Error;
use sqlx::{query_as, FromRow, Sqlite};
use sqlx_migrate::prelude::*;

#[derive(Debug, FromRow)]
pub struct Release {
    pub mbid: String,
}

pub async fn artist_credit(mut ctx: MigrationContext<'_, Sqlite>) -> Result<(), MigrationError> {
    let releases: Vec<Release> = query_as("SELECT mbid FROM releases")
        .fetch_all(ctx.tx())
        .await?;
    println!("running, {:?}", releases);
    for rel in releases {
        println!("{:?}", rel.mbid);
        let (release, tracks) = get(&rel.mbid).await.map_err(Error::msg)?;
        println!("{release:?}, {tracks:?}");
    }

    Ok(())
}
