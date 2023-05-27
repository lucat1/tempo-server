use crate::{tasks, tasks::get_queue, tasks::Task};
use base::{database::get_database, setting::TaskType};
use eyre::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn new() -> Result<JobScheduler> {
    Ok(JobScheduler::new().await?)
}

pub async fn schedule(scheduler: &mut JobScheduler, period: String, task: TaskType) -> Result<()> {
    scheduler
        .add(Job::new_async(period.as_str(), move |_, _| {
            let task = task.clone();
            Box::pin(async move {
                let result = trigger_task(&task).await;
                match result {
                    Ok(_) => tracing::info!(?task, "Scheduled tasks for the recurring job"),
                    Err(error) => tracing::warn!(%error, ?task, "Could not schedule tasks for a recurring job")
                };
            })
        })?)
        .await?;
    Ok(())
}

pub async fn trigger_task(task: &TaskType) -> Result<()> {
    let db = get_database()?;
    match task {
        TaskType::ArtistUrl => {
            let data_result = tasks::artist_url::all_data(db).await?;
            for id in data_result.into_iter() {
                get_queue().push(Task::ArtistUrl(id));
            }
            Ok(())
        }
        TaskType::ArtistDescription => {
            let data_result = tasks::artist_description::all_data(db).await?;
            for id in data_result.into_iter() {
                get_queue().push(Task::ArtistDescription(id));
            }
            Ok(())
        }
        TaskType::LastfmArtistImage => {
            let data_result = tasks::lastfm_artist_image::all_data(db).await?;
            for id in data_result.into_iter() {
                get_queue().push(Task::LastfmArtistImage(id));
            }
            Ok(())
        }
        TaskType::IndexSearch => {
            let data_result = tasks::index_search::all_data(db).await?;
            for id in data_result.into_iter() {
                get_queue().push(Task::IndexSearch(id));
            }
            Ok(())
        }
    }
}

pub async fn start(scheduler: &mut JobScheduler) -> Result<()> {
    Ok(scheduler.start().await?)
}