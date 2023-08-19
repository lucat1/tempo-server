use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use taskie_client::{Task as TaskieTask, TaskKey};
use uuid::Uuid;

use crate::fetch::musicbrainz::send_request;
use crate::tasks::TaskName;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(Uuid);

pub async fn all_data<C>(db: &C) -> Result<Vec<Data>>
where
    C: ConnectionTrait,
{
    Ok(entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| Data(a.id))
        .collect())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    pub wikipedia_extract: Option<WikipediExtract>,
}

#[derive(Deserialize)]
struct WikipediExtract {
    pub content: String,
}

#[async_trait::async_trait]
impl super::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let Data(data) = self;
        tracing::trace!(%data, "Fetching the description for artist");
        let req = Request::new(
            Method::GET,
            format!("https://musicbrainz.org/artist/{}/wikipedia-extract", data).parse()?,
        );
        let res = send_request(req).await?;
        if !res.status().is_success() {
            bail!(
                "MusicBrainz request returned non-success error code: {} {}",
                res.status(),
                res.text().await?
            );
        }
        let text = res
            .text()
            .await
            .wrap_err(eyre!("Could not read response as text"))?;

        let document: Document = serde_path_to_error::deserialize(
            &mut serde_json::Deserializer::from_str(text.as_str()),
        )
        .map_err(|e| {
            eyre!(
                "Could not parse wikipedia extract content: {} at path {}",
                e,
                e.path().to_string()
            )
        })?;
        match document.wikipedia_extract {
            Some(extract) => {
                let mut entity = entity::ArtistEntity::find_by_id(*data)
                    .one(&tx)
                    .await?
                    .ok_or(eyre!("Could not find a user with id: {}", data))?
                    .into_active_model();

                entity.description = ActiveValue::Set(Some(extract.content));
                entity
                    .save(&tx)
                    .await
                    .wrap_err(eyre!("Could not update the description of artist {}", data))?;

                tx.commit().await?;
                Ok(())
            }
            None => {
                tracing::trace!(id=%data, "Wikipedia/MusicBrainz doesn't provide a description for the artist");
                Ok(())
            }
        }
    }
}
