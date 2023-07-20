use eyre::Result;
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseTransaction, EntityTrait, IntoActiveModel,
    TransactionTrait,
};
use time::OffsetDateTime;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{
    tasks,
    tasks::{push_queue, Task, TaskData},
};
use base::{database::get_database, setting::JobType};

pub async fn new() -> Result<JobScheduler> {
    Ok(JobScheduler::new().await?)
}

pub async fn schedule(scheduler: &mut JobScheduler, period: String, task: JobType) -> Result<()> {
    let db = get_database()?;
    scheduler
        .add(Job::new_async(period.as_str(), move |_, _| {
            let task = task.clone();
            Box::pin(async move {
                match db.begin().await {
                    Ok(tx) => {
                        let result = trigger_job(tx, &task).await;
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

struct TaskDescription {
    title: String,
    description: String,
}

impl From<JobType> for TaskDescription {
    fn from(value: JobType) -> Self {
        match value {
            JobType::ArtistUrl => TaskDescription {
                title: "Fetch artist URLs".to_string(),
                description: "Fetch the URLs for all the artists".to_string(),
            },
            JobType::IndexSearch => TaskDescription {
                title: "Refresh the search index".to_string(),
                description: "Refresh the indexes used when searching".to_string(),
            },
            JobType::ArtistDescription => TaskDescription {
                title: "Fetch artist descriptions".to_string(),
                description:
                    "Fetches descriptions for all the artists from MusicBrainz (wikimedia)"
                        .to_string(),
            },
            JobType::LastfmArtistImage => TaskDescription {
                title: "Fetch artist images".to_string(),
                description: "Fetches artist images from last.fm".to_string(),
            },
        }
    }
}

pub async fn schedule_tasks(
    db: DatabaseTransaction,
    job: i64,
    tasks: Vec<TaskData>,
    depend_on: &[i64],
) -> Result<Vec<i64>> {
    let len = tasks.len();
    let db_tasks = tasks
        .iter()
        .map(|task| -> Result<entity::TaskActive> {
            Ok(entity::TaskActive {
                data: ActiveValue::Set(serde_json::to_value(task)?),

                scheduled_at: ActiveValue::Set(OffsetDateTime::now_utc()),
                job: ActiveValue::Set(job),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let res = entity::TaskEntity::insert_many(db_tasks).exec(&db).await?;
    if !depend_on.is_empty() {
        let mut dependencies = Vec::new();
        for i in 0..tasks.len() {
            let id = res.last_insert_id - (len - i - 1) as i64;
            for parent_task in depend_on.iter() {
                dependencies.push(entity::TaskDepTaskActive {
                    parent_task: ActiveValue::Set(*parent_task),
                    child_task: ActiveValue::Set(id),
                });
            }
        }
        entity::TaskDepTaskEntity::insert_many(dependencies)
            .exec(&db)
            .await?;
    }
    db.commit().await?;

    let mut ids = Vec::with_capacity(tasks.len());
    for (i, task) in tasks.into_iter().rev().enumerate() {
        let id = res.last_insert_id - (len - i - 1) as i64;
        push_queue(Task {
            id: Some(id),
            data: task,
        });
        ids.push(id);
    }
    Ok(ids)
}

pub async fn trigger_job(db: DatabaseTransaction, task: &JobType) -> Result<entity::Job> {
    let TaskDescription { title, description } = (*task).into();

    let job_active = entity::JobActive {
        id: ActiveValue::NotSet,
        title: ActiveValue::Set(title),
        description: ActiveValue::Set(Some(description)),
        scheduled_at: ActiveValue::Set(OffsetDateTime::now_utc()),
    };
    let job = job_active.insert(&db).await?;

    let mut tasks = Vec::new();
    match task {
        JobType::ArtistUrl => {
            let data = tasks::artist_url::all_data(&db).await?;
            for task in data.into_iter() {
                tasks.push(TaskData::ArtistUrl(task));
            }
        }
        JobType::ArtistDescription => {
            let data = tasks::artist_description::all_data(&db).await?;
            for task in data.into_iter() {
                tasks.push(TaskData::ArtistDescription(task));
            }
        }
        JobType::LastfmArtistImage => {
            let data = tasks::lastfm_artist_image::all_data(&db).await?;
            for task in data.into_iter() {
                tasks.push(TaskData::LastFMArtistImage(task));
            }
        }
        JobType::IndexSearch => {
            let data = tasks::index_search::all_data(&db).await?;
            for task in data.into_iter() {
                tasks.push(TaskData::IndexSearch(task));
            }
        }
    };

    schedule_tasks(db, job.id, tasks, &[]).await?;
    Ok(job)
}

pub async fn start(scheduler: &mut JobScheduler) -> Result<()> {
    Ok(scheduler.start().await?)
}
