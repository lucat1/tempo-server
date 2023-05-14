pub mod artist_description;
pub mod artist_url;
pub mod lastfm_artist_image;

use base::{database::get_database, setting::get_settings};
use deadqueue::unlimited::Queue;
use eyre::Result;
use lazy_static::lazy_static;
use sea_orm::DbConn;
use std::sync::Arc;

lazy_static! {
    pub static ref QUEUE: Arc<Queue<Task>> = Arc::new(Queue::new());
}

pub fn get_queue() -> Arc<Queue<Task>> {
    QUEUE.clone()
}

#[derive(Debug, Clone)]
pub enum Task {
    ArtistDescription(artist_description::Data),
    ArtistUrl(artist_url::Data),
    LastfmArtistImage(lastfm_artist_image::Data),
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
                tracing::trace!(%worker, ?task, "Executing task");
                match run_task(db, task.clone()).await {
                    Ok(_) => tracing::info!(%worker, ?task, "Task completed"),
                    Err(error) => tracing::warn!(%worker, ?task, %error, "Task failed with error"),
                }
            }
        });
    }
    Ok(())
}

async fn run_task(db: &DbConn, task: Task) -> Result<()> {
    match task {
        Task::ArtistDescription(data) => artist_description::run(db, data).await,
        Task::ArtistUrl(data) => artist_url::run(db, data).await,
        Task::LastfmArtistImage(data) => lastfm_artist_image::run(db, data).await,
    }
}
