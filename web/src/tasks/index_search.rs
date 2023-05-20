use eyre::{eyre, Result};
use sea_orm::{DbConn, EntityTrait, LoaderTrait, ModelTrait};
use uuid::Uuid;

use crate::search::{documents, INDEX_WRITERS};

#[derive(Debug, Clone)]
pub enum Data {
    Artist(Uuid),
    Track(Uuid),
    Release(Uuid),
}

pub async fn all_data(db: &DbConn) -> Result<Vec<Data>> {
    let artists = entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| Data::Artist(a.id));
    let tracks = entity::TrackEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| Data::Track(a.id));
    let releases = entity::ReleaseEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| Data::Release(a.id));
    Ok(artists.chain(tracks).chain(releases).collect())
}

pub async fn run(db: &DbConn, data: Data) -> Result<()> {
    let mut writers_cell = INDEX_WRITERS.lock().await;
    let writer = writers_cell
        .get_mut()
        .ok_or(eyre!("Could not get index writers"))?;
    match data {
        Data::Artist(id) => {
            let artist = entity::ArtistEntity::find_by_id(id)
                .one(db)
                .await?
                .ok_or(eyre!("Missing expected artist to be indexed: {}", id))?;

            writer
                .artists
                .add_document(documents::artist_to_document(artist)?)?;
            writer.artists.commit()?;
        }
        Data::Track(id) => {
            let track = entity::TrackEntity::find_by_id(id)
                .one(db)
                .await?
                .ok_or(eyre!("Missing expected track to be indexed: {}", id))?;
            let artist_credits = track
                .find_related(entity::ArtistCreditEntity)
                .all(db)
                .await?;
            let artists = artist_credits.load_one(entity::ArtistEntity, db).await?;
            let mut artists_data = Vec::new();
            for (i, artist_credit) in artist_credits.into_iter().enumerate() {
                let artist = artists
                    .get(i)
                    .and_then(|a| a.to_owned())
                    .ok_or(eyre!("Could not find artist related to track"))?;
                artists_data.push((artist_credit, artist));
            }

            writer
                .tracks
                .add_document(documents::track_to_document((track, artists_data))?)?;
            writer.tracks.commit()?;
        }
        Data::Release(id) => {
            let release = entity::ReleaseEntity::find_by_id(id)
                .one(db)
                .await?
                .ok_or(eyre!("Missing expected release to be indexed: {}", id))?;
            let artist_credits = release
                .find_related(entity::ArtistCreditEntity)
                .all(db)
                .await?;
            let artists = artist_credits.load_one(entity::ArtistEntity, db).await?;
            let mut artists_data = Vec::new();
            for (i, artist_credit) in artist_credits.into_iter().enumerate() {
                let artist = artists
                    .get(i)
                    .and_then(|a| a.to_owned())
                    .ok_or(eyre!("Could not find artist related to release"))?;
                artists_data.push((artist_credit, artist));
            }

            writer
                .tracks
                .add_document(documents::release_to_document((release, artists_data))?)?;
            writer.releases.commit()?;
        }
    }
    Ok(())
}
