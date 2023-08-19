use eyre::{bail, eyre, Result, WrapErr};
use levenshtein::levenshtein;
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
use base::setting::{get_settings, ArtProvider, Settings};
use entity::full::ArtistInfo;

static MAX_COVER_SIZE: u64 = 5000 * 5000;

fn in_range(val: f32, min: f32, max: f32) -> f32 {
    val / (max - min)
}

fn valuate_cover(settings: &Settings, levenshtein: f32, cover: &entity::import::Cover) -> f32 {
    let provider_index = settings
        .library
        .art
        .providers
        .iter()
        .position(|p| *p == cover.provider)
        .unwrap();

    in_range(
        provider_index as f32,
        0.0,
        settings.library.art.providers.len() as f32,
    ) * settings.library.art.provider_relevance
        + levenshtein * settings.library.art.match_relevance
        + in_range(
            (cover.width * cover.height) as f32,
            0.0,
            MAX_COVER_SIZE as f32,
        ) * settings.library.art.size_relevance
}

pub fn rank_covers(
    settings: &Settings,
    covers: &[entity::import::Cover],
    full_release: &entity::full::FullRelease,
) -> Vec<f32> {
    let release = full_release.get_release();
    covers
        .iter()
        .flat_map(|cover| {
            let joined_artists = full_release.get_joined_artists().ok()?;
            let mut distance = 1.0
                - ((levenshtein(cover.title.as_str(), release.title.as_str())
                    + levenshtein(cover.artist.as_str(), joined_artists.as_str()))
                    as f32
                    / (cover.title.len().max(release.title.len())
                        + cover.artist.len().max(joined_artists.len()))
                        as f32);
            if cover.provider == ArtProvider::CoverArtArchive {
                distance = 0.9; // TODO: better way? otherwise art from the CoverArtArchive always
                                // achieves the best score
            }
            Some(valuate_cover(settings, distance, &cover))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, _task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        let settings = get_settings()?;
        let full_release = entity::full::FullRelease::new(
            Arc::new(import.clone()),
            import
                .get_best_release_id()
                .ok_or(eyre!("Trying to rank covers with unrated releases"))?,
        )?;
        tracing::info!(id = %import.id, "Ranking covers for import");

        let ratings = rank_covers(settings, &import.covers.0, &full_release);
        tracing::info!(?ratings, "Got the ratings");

        let mut import_active = import.into_active_model();
        import_active.cover_ratings = ActiveValue::Set(entity::import::CoverRatings(ratings));
        import_active.update(&tx).await?;
        Ok(())
    }
}
