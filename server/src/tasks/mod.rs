pub mod artist_description;
pub mod artist_url;
pub mod import;
pub mod index_search;
pub mod lastfm_artist_image;
pub mod scrobble;

use async_once_cell::OnceCell;
use base::{database::get_database, setting::get_settings};
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Sub, sync::Arc};
use taskie_client::{Client, Execution, InsertTask, Task, TaskKey};
use tokio::{sync::mpsc, time::timeout};

#[async_trait::async_trait]
pub trait TaskTrait: Debug {
    async fn run<C>(&self, db: &C, task: Task<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait;
}

#[async_trait::async_trait]
pub trait TaskEntities {
    async fn all<C>(db: &C) -> Result<Vec<Self>>
    where
        C: ConnectionTrait,
        Self: Sized;

    async fn outdated<C>(db: &C) -> Result<Vec<Self>>
    where
        C: ConnectionTrait,
        Self: Sized;
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskName {
    Scrobble,
    ArtistUrl,
    IndexSearch,
    ArtistDescription,
    LastFMArtistImage,

    ImportFetch,
    ImportFetchRelease,
    ImportRankReleases,
    ImportFetchCovers,
    ImportRankCovers,
    ImportPopulate,
    ImportTrack,
}

lazy_static! {
    pub static ref TASKIE_CLIENT: Arc<OnceCell<Client>> = Arc::new(OnceCell::new());
}

pub async fn open_taskie_client() -> Result<Client> {
    let taskie_url = &get_settings()?.taskie;
    Ok(Client::new(taskie_url.clone()))
}

pub fn get_taskie_client() -> Result<&'static Client> {
    TASKIE_CLIENT
        .get()
        .ok_or(eyre!("Taskie client uninitialized"))
}

pub async fn push(tasks: &[InsertTask<TaskName>]) -> Result<Vec<Task<TaskName, TaskKey>>> {
    let client = get_taskie_client()?;
    Ok(client.push::<TaskName, TaskKey>(tasks).await?)
}

async fn run_task<C>(db: &C, task: Task<TaskName, TaskKey>) -> Result<()>
where
    C: ConnectionTrait + TransactionTrait,
{
    match &task.name {
        TaskName::Scrobble => {
            serde_json::from_value::<scrobble::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ArtistUrl => {
            serde_json::from_value::<artist_url::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::IndexSearch => {
            serde_json::from_value::<index_search::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ArtistDescription => {
            serde_json::from_value::<artist_description::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::LastFMArtistImage => {
            serde_json::from_value::<lastfm_artist_image::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }

        TaskName::ImportFetch => {
            serde_json::from_value::<import::fetch::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ImportFetchRelease => {
            serde_json::from_value::<import::fetch_release::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ImportRankReleases => {
            serde_json::from_value::<import::rank_releases::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ImportFetchCovers => {
            serde_json::from_value::<import::fetch_covers::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ImportRankCovers => {
            serde_json::from_value::<import::rank_covers::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ImportPopulate => {
            serde_json::from_value::<import::populate::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
        TaskName::ImportTrack => {
            serde_json::from_value::<import::track::Data>(task.payload.clone().into())?
                .run(db, task)
                .await?
        }
    };
    Ok(())
}

pub fn queue_loop() -> Result<()> {
    let workers = std::cmp::max(get_settings()?.tasks.workers, 1);
    tracing::info!(%workers,"Starting worker pool for background tasks");
    for worker in 0..workers {
        let db = get_database()?;
        let taskie_client = get_taskie_client()?;
        tokio::spawn(async move {
            loop {
                match taskie_client.pop::<TaskName, TaskKey>().await {
                    Ok(Execution { task, deadline }) => {
                        let id = task.id.clone();
                        tracing::trace!(%worker, ?id, ?task, ?deadline, "Executing task");

                        let (sender, mut receiver) = mpsc::channel(1);
                        tokio::spawn(async move {
                            let id = task.id.clone();
                            match sender.send(run_task(db, task).await).await {
                                Ok(_) => {
                                    tracing::trace!(%worker, %id, "Task completed, sent signal")
                                }
                                Err(err) => {
                                    tracing::warn!(%worker, %id, %err, "Could not send task completion signal")
                                }
                            };
                        });
                        let duration = deadline.sub(time::OffsetDateTime::now_utc());
                        match timeout(duration.unsigned_abs(), receiver.recv()).await {
                            Ok(Some(Ok(_))) => {
                                tracing::info!(%worker, ?id, "Task completed");
                                if let Err(err) = taskie_client.complete(id.clone()).await {
                                    tracing::error!(%worker, ?id, %err, "Could not send task completion event to the taskie server");
                                }
                            }
                            Ok(Some(Err(err))) => tracing::warn!(%worker, ?id, %err, "Task failed"),
                            Ok(None) => tracing::warn!(%worker, ?id, "Task chanel closed"),
                            Err(err) => {
                                tracing::warn!(%worker, ?id, %err, "Task timed out")
                            }
                        }
                    }
                    Err(err) => {
                        tracing::error!(%worker, ?err, "Taskie executor failed");
                        break;
                    }
                }
            }
        });
    }
    Ok(())
}
