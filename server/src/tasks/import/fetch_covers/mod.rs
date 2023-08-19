mod cover_art_archive;
mod itunes;

use base::setting::get_settings;
use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use taskie_client::{InsertTask, Task as TaskieTask, TaskKey};
use time::Duration;
use uuid::Uuid;

use crate::{
    fetch::musicbrainz::{self, MB_BASE_URL},
    import::{CombinedSearchResults, UNKNOWN_ARTIST},
    tasks::{push, TaskName},
};
use base::setting::ArtProvider;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        tracing::info!(id = %import.id, "Fetching covers for import");
        let rc_import = Arc::new(import.clone());

        let settings = get_settings()?;
        let release = entity::full::FullRelease::new(
            rc_import,
            import
                .get_best_release_id()
                .ok_or(eyre!("Trying to fetch covers with unrated releases"))?,
        )?;
        let mut covers = vec![];
        for provider in settings.library.art.providers.iter() {
            let res = match *provider {
                ArtProvider::CoverArtArchive => cover_art_archive::search(settings, &release).await,
                ArtProvider::Itunes => itunes::search(&release).await,
                ArtProvider::Deezer => Err(eyre!("missing")), //deezer::fetch(release).await,
            };
            match res {
                Ok(r) => {
                    tracing::info!(count = %r.len(), %provider, "Found cover arts");
                    covers.extend(r)
                }
                Err(err) => {
                    tracing::warn! {%provider, %err, "Error while fetching image from provider"}
                }
            }
        }

        let mut import_active = import.into_active_model();
        import_active.covers = ActiveValue::Set(entity::import::Covers(covers));
        import_active.update(&tx).await?;
        Ok(tx.commit().await?)
    }
}
