pub mod artist_description;
pub mod artist_url;
pub mod index_search;
pub mod lastfm_artist_image;
pub mod scrobble;

use base::{database::get_database, setting::get_settings};
use deadqueue::unlimited::Queue;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DbConn, EntityTrait, IntoActiveModel,
    TransactionTrait, TryIntoModel,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use time::OffsetDateTime;

use crate::tasks;

#[async_trait::async_trait]
pub trait TaskTrait: Debug {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    pub id: Option<i64>,
    pub data: TaskData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TaskData {
    Scrobble(tasks::scrobble::Task),
    ArtistUrl(tasks::artist_url::Task),
    IndexSearch(tasks::index_search::Task),
    ArtistDescription(tasks::artist_description::Task),
    LastFMArtistImage(tasks::lastfm_artist_image::Task),
}

impl TaskData {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait,
    {
        match self {
            TaskData::Scrobble(task) => task.run(db).await,
            TaskData::ArtistUrl(task) => task.run(db).await,
            TaskData::IndexSearch(task) => task.run(db).await,
            TaskData::ArtistDescription(task) => task.run(db).await,
            TaskData::LastFMArtistImage(task) => task.run(db).await,
        }
    }
}

lazy_static! {
    pub static ref QUEUE: Arc<Queue<Task>> = Arc::new(Queue::new());
}

pub fn get_queue() -> Arc<Queue<Task>> {
    QUEUE.clone()
}

pub fn push_queue(task: Task) {
    QUEUE.push(task);
}

async fn run_task<C>(db: &C, task: Task) -> Result<()>
where
    C: ConnectionTrait,
{
    let mut task_model: Option<entity::TaskActive> = None;
    if let Some(id) = task.id {
        let mut task = entity::TaskEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(eyre!("Task {} not found", id))?
            .into_active_model();
        task.started_at = ActiveValue::Set(Some(OffsetDateTime::now_utc()));
        task_model = Some(task);
    }
    task.data.run(db).await?;
    if let Some(mut task) = task_model {
        task.ended_at = ActiveValue::Set(Some(OffsetDateTime::now_utc()));
        task.update(db).await?;
    }
    Ok(())
}

pub fn queue_loop() -> Result<()> {
    let workers = std::cmp::max(get_settings()?.tasks.workers, 1);
    tracing::info!(%workers,"Starting worker pool for background tasks");
    for worker in 0..workers {
        let queue = QUEUE.clone();
        let db = get_database()?;
        tokio::spawn(async move {
            loop {
                let task = queue.pop().await;
                match db.begin().await {
                    Ok(tx) => {
                        let id = task.id;
                        tracing::trace!(%worker, ?id, ?task, "Executing task");
                        match run_task(&tx, task).await {
                            Ok(_) => tracing::info!(%worker, ?id, "Task completed"),
                            Err(error) => {
                                tracing::warn!(%worker, ?id, %error, "Task failed with error")
                            }
                        }
                        match tx.commit().await {
                            Ok(_) => {
                                tracing::trace!(%worker, ?id, "Successfully committed transaction")
                            }
                            Err(error) => {
                                tracing::error!(%worker, ?id, %error, "Could not commit transaction for task")
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!(%worker, ?task, %error, "Could not begin transaction for task")
                    }
                };
            }
        });
    }
    Ok(())
}
