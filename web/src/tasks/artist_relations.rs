use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{ActiveModelTrait, ActiveValue, DbConn, EntityTrait, IntoActiveModel};
use serde::Deserialize;
use uuid::Uuid;

use crate::fetch::musicbrainz::{send_request, MB_BASE_URL};
use base::setting::get_settings;

pub type Data = Uuid;

pub async fn all_data(db: &DbConn) -> Result<Vec<Data>> {
    Ok(entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| a.id as Data)
        .collect())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Document {
    Data(DataDocument),
    Error(ErrorDocument),
}

#[derive(Deserialize)]
struct DataDocument {
    pub content: String,
}

#[derive(Deserialize)]
struct ErrorDocument {
    pub error: usize,
    pub message: String,
}

pub async fn run(db: &DbConn, data: Data) -> Result<()> {
    tracing::debug!(%data, "Fetching artist relations");
    let req = Request::new(
        Method::GET,
        format!(
            "{}artist/{}?fmt=json&inc={}",
            MB_BASE_URL,
            data,
            ["artist-rels", "url-rels",].join("+")
        )
        .parse()?,
    );
    let res = send_request(req).await?;
    if !res.status().is_success() {
        bail!(
            "Last.fm request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let text = res.text().await?;
    tracing::info!(%text, "Got answer");
    // let document: Document = res
    //     .json()
    //     .await
    //     .wrap_err(eyre!("Could not parse last.fm api response"))?;
    Ok(())
}
