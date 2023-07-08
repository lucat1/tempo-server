pub mod artist_description;
pub mod artist_url;
pub mod index_search;
pub mod lastfm_artist_image;
pub mod scrobble;

use base::{database::get_database, setting::get_settings};
use deadqueue::unlimited::Queue;
use eyre::Result;
use lazy_static::lazy_static;
use sea_orm::DbConn;
use std::fmt::Debug;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait Task: Debug {
    async fn run(&self, db: &DbConn) -> Result<()>;
}

lazy_static! {
    pub static ref QUEUE: Arc<Queue<Box<dyn Task + Send + Sync>>> = Arc::new(Queue::new());
}

pub fn get_queue() -> Arc<Queue<Box<dyn Task + Send + Sync>>> {
    QUEUE.clone()
}

pub fn push_queue<T>(task: T)
where
    T: Task + Send + Sync + 'static,
{
    QUEUE.push(Box::new(task));
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
                match task.run(db).await {
                    Ok(_) => tracing::info!(%worker, ?task, "Task completed"),
                    Err(error) => tracing::warn!(%worker, ?task, %error, "Task failed with error"),
                }
            }
        });
    }
    Ok(())
}
