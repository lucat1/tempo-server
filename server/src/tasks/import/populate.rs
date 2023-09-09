use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use strfmt::strfmt;
use taskie_client::{InsertTask, Task as TaskieTask, TaskKey};
use time::Duration;
use uuid::Uuid;

use crate::{
    fetch::musicbrainz::{self, MB_BASE_URL},
    import::{CombinedSearchResults, UNKNOWN_ARTIST},
    tasks::{push, TaskName},
};
use base::{setting::get_settings, util::path_to_str};
use entity::full::{ArtistInfo, GetArtistCredits};
use tag::{sanitize_map, tag_to_string_map, tags_from_full_release};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let settings = get_settings()?;
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        let import_rc = Arc::new(import);
        let full_release = entity::full::FullRelease::new(
            import_rc.clone(),
            import_rc
                .selected_release
                .ok_or(eyre!("Trying to process a loading import"))?,
        )?;

        let release_root = settings.library.path.join(PathBuf::from_str(
            strfmt(
                settings.library.release_name.as_str(),
                &sanitize_map(tag_to_string_map(&tags_from_full_release(&full_release)?)),
            )?
            .as_str(),
        )?);

        // Save the release and its relationships
        let mut release = full_release.get_release().clone();
        release.path = Some(path_to_str(&release_root)?);
        entity::ReleaseEntity::insert(release.into_active_model())
            .exec(&tx)
            .await?;
        let artists: Vec<_> = full_release
            .get_artists()?
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistEntity::insert_many(artists).exec(&tx).await?;
        let artist_credits: Vec<_> = full_release
            .get_artist_credits()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistCreditEntity::insert_many(artist_credits)
            .exec(&tx)
            .await?;
        let artist_credits_release: Vec<_> = full_release
            .get_artist_credits_release()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistCreditReleaseEntity::insert_many(artist_credits_release)
            .exec(&tx)
            .await?;

        // Save mediums
        let mediums: Vec<_> = full_release
            .get_mediums()
            .into_iter()
            .map(|m| m.clone().into_active_model())
            .collect();
        entity::MediumEntity::insert_many(mediums).exec(&tx).await?;

        // Save tracks
        let full_tracks = full_release.get_full_tracks()?;
        for full_track in full_tracks.into_iter() {
            let track = full_track.get_track().clone().into_active_model();
            entity::TrackEntity::insert(track).exec(&tx).await?;
            let artists: Vec<_> = full_track
                .get_artists()?
                .into_iter()
                .map(|a| a.clone().into_active_model())
                .collect();
            entity::ArtistEntity::insert_many(artists).exec(&tx).await?;
            let artist_credits: Vec<_> = full_track
                .get_artist_credits()
                .into_iter()
                .map(|a| a.clone().into_active_model())
                .collect();
            entity::ArtistCreditEntity::insert_many(artist_credits)
                .exec(&tx)
                .await?;
            let artist_credits_track: Vec<_> = full_track
                .get_artist_credits_track()
                .into_iter()
                .map(|a| a.clone().into_active_model())
                .collect();
            entity::ArtistCreditTrackEntity::insert_many(artist_credits_track)
                .exec(&tx)
                .await?;
            let artists: Vec<_> = full_track
                .get_related_artists()?
                .into_iter()
                .map(|a| a.clone().into_active_model())
                .collect();
            entity::ArtistEntity::insert_many(artists).exec(&tx).await?;
            let artist_relations: Vec<_> = full_track
                .get_relations()
                .into_iter()
                .map(|a| a.clone().into_active_model())
                .collect();
            entity::ArtistTrackRelationEntity::insert_many(artist_relations)
                .exec(&tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
