pub mod artist_description;
pub mod artist_url;
pub mod import;
pub mod index_search;
pub mod lastfm_artist_image;
pub mod scrobble;

use base::{database::get_database, setting::get_settings};
use deadqueue::unlimited::Queue;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    JoinType, ModelTrait, PaginatorTrait, QueryFilter, QuerySelect, RelationTrait,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc, time::Duration};
use time::OffsetDateTime;
use tokio::time::sleep;

use crate::tasks;

#[async_trait::async_trait]
pub trait TaskTrait: Debug {
    async fn run<D>(&self, db: &D, id: Option<i64>) -> Result<()>
    where
        D: ConnectionTrait + TransactionTrait;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    pub id: Option<i64>,
    pub data: TaskData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum TaskData {
    Scrobble(tasks::scrobble::Task),
    ArtistUrl(tasks::artist_url::Task),
    IndexSearch(tasks::index_search::Task),
    ArtistDescription(tasks::artist_description::Task),
    LastFMArtistImage(tasks::lastfm_artist_image::Task),

    ImportFetch(tasks::import::fetch::Task),
    ImportFetchRelease(tasks::import::fetch_release::Task),
}

impl Task {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait + TransactionTrait,
    {
        match &self.data {
            TaskData::Scrobble(task) => task.run(db, self.id).await,
            TaskData::ArtistUrl(task) => task.run(db, self.id).await,
            TaskData::IndexSearch(task) => task.run(db, self.id).await,
            TaskData::ArtistDescription(task) => task.run(db, self.id).await,
            TaskData::LastFMArtistImage(task) => task.run(db, self.id).await,

            TaskData::ImportFetch(task) => task.run(db, self.id).await,
            TaskData::ImportFetchRelease(task) => task.run(db, self.id).await,
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
    C: ConnectionTrait + TransactionTrait,
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
    task.run(db).await?;
    if let Some(mut task) = task_model {
        task.ended_at = ActiveValue::Set(Some(OffsetDateTime::now_utc()));
        task.update(db).await?;
    }
    Ok(())
}

async fn check_task_deps<C>(db: &C, task_id: i64) -> Result<bool>
where
    C: ConnectionTrait + TransactionTrait,
{
    let non_ended_dependencies = entity::TaskEntity::find()
        .join(
            JoinType::InnerJoin,
            entity::TaskDepTaskRelation::ChildTask.def(),
        )
        .count(db)
        .await?;
    // let task = entity::TaskEntity::find_by_id(task_id)
    //     .one(db)
    //     .await?
    //     .ok_or(eyre!("Task {} not found", task_id))?;
    // let non_ended_dependencies = task
    //     .find_related(entity::TaskEntity)
    //     .filter(entity::TaskColumn::EndedAt.is_not_null())
    //     .count(db)
    //     .await?;

    Ok(non_ended_dependencies == 0)
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
                let id = task.id;
                tracing::trace!(%worker, ?id, ?task, "Executing task");

                let mut should_run = true;
                if let Some(id) = task.id {
                    match check_task_deps(db, id).await {
                        Ok(run) => {
                            should_run = run;
                            if run {
                                tracing::trace!(%worker, ?id, "Task can run")
                            } else {
                                tracing::trace!(%worker, ?id, "Task is blocked waiting on another task");
                                sleep(Duration::from_millis(500)).await;
                                queue.push(task.clone());
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%worker, ?id, %error, "Could not check task dependencies")
                        }
                    }
                }

                if should_run {
                    match run_task(db, task).await {
                        Ok(_) => tracing::info!(%worker, ?id, "Task completed"),
                        Err(error) => {
                            tracing::warn!(%worker, ?id, %error, "Task failed with error")
                        }
                    }
                }
            }
        });
    }
    Ok(())
}
