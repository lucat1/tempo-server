use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use taskie_client::{Task as TaskieTask, TaskKey};
use uuid::Uuid;

use crate::{
    fetch::musicbrainz::{self, MB_BASE_URL},
    import::SearchResult,
    tasks::TaskName,
};
use base::util::dedup;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data {
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
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let mut import = entity::ImportEntity::find_by_id(self.import_id)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        let release = fetch_release(self.release_id).await?;

        let mut import_active = import.clone().into_active_model();
        import.artists.0.extend(release.artists);
        import_active.artists = ActiveValue::Set(entity::import::Artists(dedup(import.artists.0)));

        import.artist_credits.0.extend(release.artist_credits);
        import_active.artist_credits = ActiveValue::Set(entity::import::ArtistCredits(dedup(
            import.artist_credits.0,
        )));

        import.releases.0.push(release.release);
        import_active.releases = ActiveValue::Set(entity::import::Releases(import.releases.0));

        import.mediums.0.extend(release.mediums);
        import_active.mediums = ActiveValue::Set(entity::import::Mediums(import.mediums.0));

        import.tracks.0.extend(release.tracks);
        import_active.tracks = ActiveValue::Set(entity::import::Tracks(dedup(import.tracks.0)));

        import
            .artist_track_relations
            .0
            .extend(release.artist_track_relations);
        import_active.artist_track_relations = ActiveValue::Set(
            entity::import::ArtistTrackRelations(import.artist_track_relations.0),
        );

        import
            .artist_credit_releases
            .0
            .extend(release.artist_credit_releases);
        import_active.artist_credit_releases = ActiveValue::Set(
            entity::import::ArtistCreditReleases(import.artist_credit_releases.0),
        );

        import
            .artist_credit_tracks
            .0
            .extend(release.artist_credit_tracks);
        import_active.artist_credit_tracks = ActiveValue::Set(entity::import::ArtistCreditTracks(
            import.artist_credit_tracks.0,
        ));

        import_active.update(&tx).await?;
        tx.commit().await?;

        Ok(())
    }
}
