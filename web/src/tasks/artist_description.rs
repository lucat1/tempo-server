use eyre::Result;
use sea_orm::{DbConn, EntityTrait};
use uuid::Uuid;

pub async fn all_data(db: &DbConn) -> Result<Vec<Data>> {
    Ok(entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| a.id as Data)
        .collect())
}

pub type Data = Uuid;

pub async fn run(data: Data) -> Result<()> {
    tracing::debug!(%data, "Fetching the description for artist");
    Ok(())
}
