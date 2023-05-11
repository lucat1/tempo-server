use common::fetch::{CLIENT, MB_USER_AGENT};
use eyre::{bail, eyre, Result, WrapErr};
use reqwest::header::USER_AGENT;
use sea_orm::{ActiveModelTrait, ActiveValue, DbConn, EntityTrait, IntoActiveModel};
use serde::Deserialize;
use uuid::Uuid;

pub async fn all_data(db: &DbConn) -> Result<Vec<Data>> {
    Ok(entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| a.id as Data)
        .collect())
}

pub type Data = Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    pub wikipedia_extract: Option<WikipediExtract>,
}

#[derive(Deserialize)]
struct WikipediExtract {
    pub content: String,
}

pub async fn run(db: &DbConn, data: Data) -> Result<()> {
    tracing::debug!(%data, "Fetching the description for artist");
    let res = CLIENT
        .get(format!(
            "https://musicbrainz.org/artist/{}/wikipedia-extract",
            data
        ))
        .header(USER_AGENT, MB_USER_AGENT)
        .send()
        .await?;
    if !res.status().is_success() {
        bail!(
            "Musicbrainz request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let document: Document = res
        .json()
        .await
        .wrap_err(eyre!("Could not read parse wikipedia extract content"))?;
    match document.wikipedia_extract {
        Some(extract) => {
            let mut entity = entity::ArtistEntity::find_by_id(data)
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
            tracing::debug!(id=%data, "Wikipedia/MusicBrainz doesn't provide a description for the artist");
            Ok(())
        }
    }
}
