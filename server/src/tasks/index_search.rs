use eyre::{eyre, Result};
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait};
use serde::{Deserialize, Serialize};

use crate::search::{documents, INDEX_WRITERS};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Task {
    Artists,
    Tracks,
    Releases,
}

pub async fn all_data<C>(_: &C) -> Result<Vec<Task>>
where
    C: ConnectionTrait,
{
    Ok(vec![Task::Artists, Task::Tracks, Task::Releases])
}

#[async_trait::async_trait]
impl super::TaskTrait for Task {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait,
    {
        let mut writers_cell = INDEX_WRITERS.lock().await;
        let writer = writers_cell
            .get_mut()
            .ok_or(eyre!("Could not get index writers"))?;
        match self {
            Task::Artists => {
                let artists = entity::ArtistEntity::find().all(db).await?;

                for artist in artists.into_iter() {
                    writer
                        .artists
                        .add_document(documents::artist_to_document(artist)?)?;
                }
                writer.artists.commit()?;
            }
            Task::Tracks => {
                let tracks = entity::TrackEntity::find().all(db).await?;
                let tracks_artist_credits = tracks
                    .load_many_to_many(
                        entity::ArtistCreditEntity,
                        entity::ArtistCreditTrackEntity,
                        db,
                    )
                    .await?;
                for (i, track) in tracks.into_iter().enumerate() {
                    let artist_credits = tracks_artist_credits.get(i).ok_or(eyre!(
                        "Track {} ({}) doesn't have any associated artist credits",
                        i,
                        track.id
                    ))?;
                    let artists = artist_credits.load_one(entity::ArtistEntity, db).await?;
                    let mut artists_self = Vec::new();
                    for (i, artist_credit) in artist_credits.iter().enumerate() {
                        let artist = artists
                            .get(i)
                            .and_then(|a| a.to_owned())
                            .ok_or(eyre!("Could not find artist related to track"))?;
                        artists_self.push((artist_credit.to_owned(), artist));
                    }

                    writer
                        .tracks
                        .add_document(documents::track_to_document((track, artists_self))?)?;
                }
                writer.tracks.commit()?;
            }
            Task::Releases => {
                let releases = entity::ReleaseEntity::find().all(db).await?;
                let tracks_artist_credits = releases
                    .load_many_to_many(
                        entity::ArtistCreditEntity,
                        entity::ArtistCreditReleaseEntity,
                        db,
                    )
                    .await?;
                for (i, release) in releases.into_iter().enumerate() {
                    let artist_credits = tracks_artist_credits.get(i).ok_or(eyre!(
                        "Release {} ({}) doesn't have any associated artist credits",
                        i,
                        release.id
                    ))?;
                    let artists = artist_credits.load_one(entity::ArtistEntity, db).await?;
                    let mut artists_self = Vec::new();
                    for (i, artist_credit) in artist_credits.iter().enumerate() {
                        let artist = artists
                            .get(i)
                            .and_then(|a| a.to_owned())
                            .ok_or(eyre!("Could not find artist related to release"))?;
                        artists_self.push((artist_credit.to_owned(), artist));
                    }

                    writer
                        .releases
                        .add_document(documents::release_to_document((release, artists_self))?)?;
                }
                writer.releases.commit()?;
            }
        }
        Ok(())
    }
}
