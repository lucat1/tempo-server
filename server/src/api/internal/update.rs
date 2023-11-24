use axum::extract::State;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use taskie_client::InsertTask;
use thiserror::Error;

use crate::api::{extract::Path, AppState, Error};
use crate::tasks::{
    artist_description, artist_url, index_search, lastfm_artist_image, push, TaskEntities, TaskName,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(untagged)]
pub enum UpdateType {
    Artist(entity::UpdateArtistType),
    Other(OtherUpdateType),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtherUpdateType {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "index_search")]
    IndexSearch,
}

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("Error while fetching task data: {0}")]
    Fetcher(eyre::Report),
}

// TODO: handle dependencies somehow
fn unpack_all(update_type: UpdateType) -> Vec<UpdateType> {
    match update_type {
        UpdateType::Other(OtherUpdateType::All) => vec![
            UpdateType::Artist(entity::UpdateArtistType::ArtistUrl),
            UpdateType::Artist(entity::UpdateArtistType::ArtistDescription),
            UpdateType::Artist(entity::UpdateArtistType::LastFMArtistImage),
            UpdateType::Other(OtherUpdateType::IndexSearch),
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

macro_rules! insert_task {
    ($db:expr, $t:ident, $pkg:ident, $op:ident) => {
        map_insert_task(
            TaskName::$t,
            $pkg::Data::$op($db).await.map_err(UpdateError::Fetcher)?,
        )
    };
}

macro_rules! insert_all_task {
    ($db:expr, $t:ident, $pkg:ident) => {
        insert_task!($db, $t, $pkg, all)
    };
}

pub async fn all(
    State(AppState(db)): State<AppState>,
    Path(update_type): Path<UpdateType>,
) -> Result<(), Error> {
    for update in unpack_all(update_type).into_iter() {
        let tasks = match update {
            UpdateType::Artist(au) => match au {
                entity::UpdateArtistType::ArtistUrl => {
                    insert_all_task!(&db, ArtistUrl, artist_url)
                }
                entity::UpdateArtistType::ArtistDescription => {
                    insert_all_task!(&db, ArtistDescription, artist_description)
                }
                entity::UpdateArtistType::LastFMArtistImage => {
                    insert_all_task!(&db, LastFMArtistImage, lastfm_artist_image)
                }
            },
            UpdateType::Other(OtherUpdateType::IndexSearch) => {
                insert_all_task!(&db, IndexSearch, index_search)
            }
            _ => unreachable!(),
        };
        tracing::info!(?tasks, "Queueing the update tasks");
        push(&tasks).await?;
    }
    Ok(())
}

macro_rules! insert_outdated_task {
    ($db:expr, $t:ident, $pkg:ident) => {
        insert_task!($db, $t, $pkg, outdated)
    };
}

pub async fn outdated(
    State(AppState(db)): State<AppState>,
    Path(update_type): Path<UpdateType>,
) -> Result<(), Error> {
    for update in unpack_all(update_type).into_iter() {
        let tasks = match update {
            UpdateType::Artist(au) => match au {
                entity::UpdateArtistType::ArtistUrl => {
                    insert_outdated_task!(&db, ArtistUrl, artist_url)
                }
                entity::UpdateArtistType::ArtistDescription => {
                    insert_outdated_task!(&db, ArtistDescription, artist_description)
                }
                entity::UpdateArtistType::LastFMArtistImage => {
                    insert_outdated_task!(&db, LastFMArtistImage, lastfm_artist_image)
                }
            },
            UpdateType::Other(OtherUpdateType::IndexSearch) => {
                insert_outdated_task!(&db, IndexSearch, index_search)
            }
            _ => unreachable!(),
        };

        tracing::info!(?tasks, "Queueing the update tasks");
        push(&tasks).await?;
    }
    Ok(())
}
