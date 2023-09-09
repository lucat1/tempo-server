use common::track::TrackFile;
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
use entity::{
    conflict::{
        ARTIST_CONFLICT, ARTIST_CREDIT_CONFLICT, ARTIST_CREDIT_RELEASE_CONFLICT,
        ARTIST_CREDIT_TRACK_CONFLICT, ARTIST_TRACK_RELATION_CONFLICT, IMAGE_CONFLICT_1,
        IMAGE_CONFLICT_2, IMAGE_RELEASE_CONFLICT, MEDIUM_CONFLICT, RELEASE_CONFLICT,
        TRACK_CONFLICT,
    },
    full::{ArtistInfo, GetArtistCredits},
};
use tag::{
    sanitize_map, tag_to_string_map, tags_from_combination, tags_from_full_release,
    tags_from_full_track, Picture, PictureType,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub import: Uuid,
    pub release: Uuid,
    pub track: Uuid,
    // TODO: we could consider making this an Option and have releases with
    // all the tracks in the DB but some missing on the FS, so that they can be
    // added later on.
    pub source: usize,
    pub cover: Option<String>,
}

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let settings = get_settings()?;
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.import)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        let release = entity::ReleaseEntity::find_by_id(self.release)
            .one(&tx)
            .await?
            .ok_or(eyre!("Release not found"))?;
        let import_rc = Arc::new(import);
        let full_track = entity::full::FullTrack::new(import_rc.clone(), self.track)?;
        let full_release = entity::full::FullRelease::new(import_rc.clone(), self.release)?;
        let internal_track = import_rc
            .source_tracks
            .0
            .get(self.source)
            .ok_or(eyre!("Invalid track mapping"))?;
        let mut file = TrackFile::open(&settings.library, &internal_track.path.parse()?)?;
        let tags = tags_from_combination(&full_release, &full_track)?;

        let release_path: PathBuf = release
            .path
            .ok_or(eyre!("Importing a track for a release without a path"))?
            .parse()?;
        let track_path = settings.library.path.join(PathBuf::from_str(
            format!(
                "{}.{}",
                strfmt(
                    settings.library.track_name.as_str(),
                    &sanitize_map(tag_to_string_map(&tags)),
                )?,
                file.format.ext()
            )
            .as_str(),
        )?);
        let track_path = release_path.join(track_path);

        file.duplicate_to(&settings.library, &track_path)
            .wrap_err(eyre!(
                "Could not copy track {:?} to its new location: {:?}",
                file.path,
                track_path
            ))?;
        if settings.library.tagging.clear {
            file.clear()
                .wrap_err(eyre!("Could not celar tracks from file: {:?}", track_path))?;
        }
        file.apply(tags)
            .wrap_err(eyre!("Could not apply new tags to track: {:?}", track_path))?;
        file.write()
            .wrap_err(eyre!("Could not write tags to track: {:?}", track_path))?;
        if let Some(cover_path_str) = &self.cover {
            let path: PathBuf = cover_path_str.parse()?;
            let data = tokio::fs::read(&path).await?;
            let pic = Picture {
                mime_type: settings.library.art.format.mime(),
                picture_type: PictureType::CoverFront,
                description: "Front Cover".to_string(),
                data,
            };
            file.set_pictures(vec![pic])
                .wrap_err(eyre!("Could not add picture tag to file: {:?}", track_path))?;
        }

        let track = full_track.get_track().clone().into_active_model();
        entity::TrackEntity::insert(track)
            .on_conflict(TRACK_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        let artists: Vec<_> = full_track
            .get_artists()?
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistEntity::insert_many(artists)
            .on_conflict(ARTIST_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        let artist_credits: Vec<_> = full_track
            .get_artist_credits()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistCreditEntity::insert_many(artist_credits)
            .on_conflict(ARTIST_CREDIT_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        let artist_credits_track: Vec<_> = full_track
            .get_artist_credits_track()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistCreditTrackEntity::insert_many(artist_credits_track)
            .on_conflict(ARTIST_CREDIT_TRACK_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        let artists: Vec<_> = full_track
            .get_related_artists()?
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistEntity::insert_many(artists)
            .on_conflict(ARTIST_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        let artist_relations: Vec<_> = full_track
            .get_relations()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistTrackRelationEntity::insert_many(artist_relations)
            .on_conflict(ARTIST_TRACK_RELATION_CONFLICT.to_owned())
            .exec(&tx)
            .await?;
        Ok(tx.commit().await?)
    }
}
