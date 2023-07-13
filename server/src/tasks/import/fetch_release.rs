use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    fetch::musicbrainz::{self, MB_BASE_URL},
    import::SearchResult,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Task {
    pub import_id: Uuid,
    pub release_id: Uuid,
}

pub async fn fetch_release(id: Uuid) -> Result<SearchResult> {
    tracing::info! {%id, "Fetching MusicBrainz id"};
    let mut url = MB_BASE_URL.join(format!("release/{}", id).as_str())?;
    url.query_pairs_mut()
        .append_pair(
            "inc",
            [
                "artists",
                "artist-credits",
                "release-groups",
                "labels",
                "recordings",
                "genres",
                "work-rels",
                "work-level-rels",
                "artist-rels",
                "recording-rels",
                "instrument-rels",
                "recording-level-rels",
            ]
            .join("+")
            .as_str(),
        )
        .append_pair("fmt", "json");
    let req = Request::new(Method::GET, url);
    let res = musicbrainz::send_request(req).await?;
    if !res.status().is_success() {
        bail!(
            "Musicbrainz request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let text = res
        .text()
        .await
        .wrap_err(eyre!("Could not read response as text"))?;

    let json_release: musicbrainz::Release =
        serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text.as_str()))
            .map_err(|e| {
                eyre!(
                    "Error while decoding JSON at path {}: {}",
                    e.path().to_string(),
                    e
                )
            })?;
    Ok(json_release.into())
}

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Task {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait,
    {
        let import = entity::ImportEntity::find_by_id(self.import_id)
            .one(db)
            .await?
            .ok_or(eyre!("Import not found"))?;
        Ok(())
    }
}
