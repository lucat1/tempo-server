use base::{database::get_database, setting::TaskType};
use eyre::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use web::{get_queue, tasks, tasks::Task};

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
        TaskType::ArtistDescription => {
            let data_result = tasks::artist_description::all_data(db).await;
            match data_result {
                Ok(data) => {
                    for id in data.into_iter() {
                        get_queue().push(Task::ArtistDescription(id));
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    }
}

pub async fn start(scheduler: &mut JobScheduler) -> Result<()> {
    Ok(scheduler.start().await?)
}
