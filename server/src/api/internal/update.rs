use axum::extract::State;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use taskie_client::InsertTask;

use crate::api::{extract::Path, jsonapi::Error, AppState};
use crate::tasks::{
    artist_description, artist_url, index_search, lastfm_artist_image, push, TaskEntities, TaskName,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(untagged)]
pub enum UpdateType {
    #[serde(rename = "all")]
    All,

    Artist(entity::UpdateArtistType),
    IndexSearch,
}

// TODO: handle dependencies somehow
fn unpack_all(update_type: UpdateType) -> Vec<UpdateType> {
    match update_type {
        UpdateType::All => vec![
            UpdateType::Artist(entity::UpdateArtistType::ArtistUrl),
            UpdateType::Artist(entity::UpdateArtistType::ArtistDescription),
            UpdateType::Artist(entity::UpdateArtistType::LastFMArtistImage),
            UpdateType::IndexSearch,
        ],
        u => vec![u],
    }
}

fn map_insert_task<T>(name: TaskName, values: Vec<T>) -> Vec<InsertTask<TaskName>>
where
    T: Serialize,
{
    values
        .into_iter()
        .map(|data| InsertTask {
            name,
            payload: Some(json!(data)),
            depends_on: vec![],
            duration: time::Duration::seconds(60),
        })
        .collect()
}

pub async fn all(
    State(AppState(db)): State<AppState>,
    Path(update_type): Path<UpdateType>,
) -> Result<(), Error> {
    for update in unpack_all(update_type).into_iter() {
        let tasks = match update {
            UpdateType::Artist(au) => match au {
                entity::UpdateArtistType::ArtistUrl => {
                    map_insert_task(TaskName::ArtistUrl, artist_url::Data::all(&db).await?)
                }
                entity::UpdateArtistType::ArtistDescription => map_insert_task(
                    TaskName::ArtistDescription,
                    artist_description::Data::all(&db).await?,
                ),
                entity::UpdateArtistType::LastFMArtistImage => map_insert_task(
                    TaskName::LastFMArtistImage,
                    lastfm_artist_image::Data::all(&db).await?,
                ),
            },
            UpdateType::IndexSearch => map_insert_task(
                TaskName::IndexSearch,
                index_search::Data::outdated(&db).await?,
            ),
            UpdateType::All => unreachable!(),
        };
        tracing::info!(?tasks, "Queueing the update tasks");
        push(&tasks).await?;
    }
    Ok(())
}

pub async fn outdated(
    State(AppState(db)): State<AppState>,
    Path(update_type): Path<UpdateType>,
) -> Result<(), Error> {
    for update in unpack_all(update_type).into_iter() {
        let tasks = match update {
            UpdateType::Artist(au) => match au {
                entity::UpdateArtistType::ArtistUrl => {
                    map_insert_task(TaskName::ArtistUrl, artist_url::Data::outdated(&db).await?)
                }
                entity::UpdateArtistType::ArtistDescription => map_insert_task(
                    TaskName::ArtistDescription,
                    artist_description::Data::outdated(&db).await?,
                ),
                entity::UpdateArtistType::LastFMArtistImage => map_insert_task(
                    TaskName::LastFMArtistImage,
                    lastfm_artist_image::Data::outdated(&db).await?,
                ),
            },
            UpdateType::IndexSearch => map_insert_task(
                TaskName::IndexSearch,
                index_search::Data::outdated(&db).await?,
            ),
            UpdateType::All => unreachable!(),
        };

        tracing::info!(?tasks, "Queueing the update tasks");
        push(&tasks).await?;
    }
    Ok(())
}
