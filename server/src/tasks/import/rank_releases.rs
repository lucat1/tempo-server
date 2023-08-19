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

        let mut rated_search_results = search_results
            .into_iter()
            .map(|search_result| {
                let rank::Rating(rating, mapping) = rank::rate_and_match(&tracks, &search_result);
                RatedSearchResult {
                    rating,
                    search_result,
                    mapping,
                }
            })
            .collect::<Vec<_>>();
        // rated_search_results
        //     .sort_by(|a, b| a.rating.partial_cmp(&b.rating).unwrap_or(Ordering::Equal));
        // let fetch::SearchResult(full_release, _) = rated_search_results
        //     .first()
        //     .map(|r| r.search_result.clone())
        //     .ok_or(eyre!("No results found"))?;

        tracing::info!(id = %import.id, "Ranking releases for import");
        Ok(())
    }
}
