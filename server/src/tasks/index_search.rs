use eyre::{eyre, Result};
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, TransactionTrait};
use serde::{Deserialize, Serialize};
use taskie_client::{Task as TaskieTask, TaskKey};

use crate::search::{documents, INDEX_WRITERS};
use crate::tasks::TaskName;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Data {
    Artists,
    Tracks,
    Releases,
}

pub async fn all_data<C>(_: &C) -> Result<Vec<Data>>
where
    C: ConnectionTrait,
{
    Ok(vec![Data::Artists, Data::Tracks, Data::Releases])
}

#[async_trait::async_trait]
impl super::TaskTrait for Data {
    async fn run<C>(&self, db: &C, _task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let mut writers_cell = INDEX_WRITERS.lock().await;
        let writer = writers_cell
            .get_mut()
            .ok_or(eyre!("Could not get index writers"))?;
        match self {
            Data::Artists => {
                let artists = entity::ArtistEntity::find().all(&tx).await?;

                for artist in artists.into_iter() {
                    writer
                        .artists
                        .add_document(documents::artist_to_document(artist)?)?;
                }
                writer.artists.commit()?;
            }
            Data::Tracks => {
                let tracks = entity::TrackEntity::find().all(&tx).await?;
                let tracks_artist_credits = tracks
                    .load_many_to_many(
                        entity::ArtistCreditEntity,
                        entity::ArtistCreditTrackEntity,
                        &tx,
                    )
                    .await?;
                for (i, track) in tracks.into_iter().enumerate() {
                    let artist_credits = tracks_artist_credits.get(i).ok_or(eyre!(
                        "Track {} ({}) doesn't have any associated artist credits",
                        i,
                        track.id
                    ))?;
                    let artists = artist_credits.load_one(entity::ArtistEntity, &tx).await?;
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
            Data::Releases => {
                let releases = entity::ReleaseEntity::find().all(&tx).await?;
                let tracks_artist_credits = releases
                    .load_many_to_many(
                        entity::ArtistCreditEntity,
                        entity::ArtistCreditReleaseEntity,
                        &tx,
                    )
                    .await?;
                for (i, release) in releases.into_iter().enumerate() {
                    let artist_credits = tracks_artist_credits.get(i).ok_or(eyre!(
                        "Release {} ({}) doesn't have any associated artist credits",
                        i,
                        release.id
                    ))?;
                    let artists = artist_credits.load_one(entity::ArtistEntity, &tx).await?;
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
