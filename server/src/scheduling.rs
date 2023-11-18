use eyre::Result;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::json;
use time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::tasks::{self, push, TaskEntities, TaskName};
use base::{database::get_database, setting::JobType};
use taskie_client::InsertTask;

pub async fn new() -> Result<JobScheduler> {
    Ok(JobScheduler::new().await?)
}

pub async fn schedule(scheduler: &mut JobScheduler, period: String, task: JobType) -> Result<()> {
    let db = get_database()?;
    scheduler
        .add(Job::new_async(period.as_str(), move |_, _| {
            Box::pin(async move {
                match db.begin().await {
                    Ok(tx) => {
                        let result = run_tasks(&tx, &task).await;
                        match result {
                            Ok(_) => tracing::info!(?task, "Scheduled tasks for the recurring job"),
                            Err(error) => tracing::warn!(%error, ?task, "Could not schedule tasks for a recurring job")
                        };
                    },
                    Err(error) => tracing::warn!(%error, ?task, "Could not begin database transation for recurring job")
                }
            })
        })?)
        .await?;
    Ok(())
}

pub async fn run_tasks<C>(db: &C, task: &JobType) -> Result<()>
where
    C: ConnectionTrait,
{
    let name = match task {
        JobType::ArtistUrl => TaskName::ArtistUrl,
        JobType::ArtistDescription => TaskName::ArtistDescription,
        JobType::IndexSearch => TaskName::IndexSearch,
        JobType::LastFMArtistImage => TaskName::LastFMArtistImage,
    };
    let data: Vec<_> = match task {
        JobType::ArtistUrl => tasks::artist_url::all_data(db)
            .await?
            .into_iter()
            .map(|data| json!(data))
            .collect(),
        JobType::ArtistDescription => tasks::artist_description::Data::all(db)
            .await?
            .into_iter()
            .map(|data| json!(data))
            .collect(),

        JobType::LastFMArtistImage => tasks::lastfm_artist_image::all_data(db)
            .await?
            .into_iter()
            .map(|data| json!(data))
            .collect(),

        JobType::IndexSearch => tasks::index_search::all_data(db)
            .await?
            .into_iter()
            .map(|data| json!(data))
            .collect(),
    };

    let tasks: Vec<_> = data
        .into_iter()
        .map(|data| InsertTask {
            name,
            payload: Some(data),
            duration: Duration::seconds(60),
            depends_on: vec![],
        })
        .collect();

    push(&tasks).await?;
    Ok(())
}

pub async fn start(scheduler: &mut JobScheduler) -> Result<()> {
    Ok(scheduler.start().await?)
}
