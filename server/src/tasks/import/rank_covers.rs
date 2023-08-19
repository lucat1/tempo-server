use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use taskie_client::{InsertTask, Task as TaskieTask, TaskKey};
use time::Duration;
use uuid::Uuid;

use crate::{
    fetch::musicbrainz::{self, MB_BASE_URL},
    import::{CombinedSearchResults, UNKNOWN_ARTIST},
    tasks::{push, TaskName},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let mut import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;

        // let covers = rank::rank_covers(library, covers_by_provider, &full_release);
        // let selected = (
        //     full_release.release.id,
        //     if !covers.is_empty() { Some(0) } else { None },
        // );

        tracing::info!(id = %import.id, "Ranking covers for import");
        Ok(())
    }
}
