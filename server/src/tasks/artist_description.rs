use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{ActiveModelTrait, ActiveValue, DbConn, EntityTrait, IntoActiveModel};
use serde::Deserialize;
use uuid::Uuid;

use crate::fetch::musicbrainz::send_request;

#[derive(Debug)]
pub struct Task(Uuid);

pub async fn all_data(db: &DbConn) -> Result<Vec<Task>> {
    Ok(entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| Task(a.id))
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
impl super::Task for Task {
    async fn run(&self, db: &DbConn) -> Result<()> {
        let Task(data) = self;
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
                    .one(db)
                    .await?
                    .ok_or(eyre!("Could not find a user with id: {}", data))?
                    .into_active_model();

                entity.description = ActiveValue::Set(Some(extract.content));
                entity
                    .save(db)
                    .await
                    .wrap_err(eyre!("Could not update the description of artist {}", data))?;

                Ok(())
            }
            None => {
                tracing::trace!(id=%data, "Wikipedia/MusicBrainz doesn't provide a description for the artist");
                Ok(())
            }
        }
    }
}
