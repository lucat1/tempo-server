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

static COUNT: u32 = 8;

pub async fn search(release: &entity::InternalRelease) -> Result<CombinedSearchResults> {
    let raw_artists = release.artists.join(", ");
    let artists = match raw_artists.as_str() {
        UNKNOWN_ARTIST => "",
        s => s,
    };
    tracing::info! {%artists, title = %release.title, "Searching for releases on MusicBrainz"};
    let mut url = MB_BASE_URL.join("release/")?;
    url.query_pairs_mut()
        .append_pair(
            "query",
            format!(
                "release:{} artist:{} tracks:{}",
                release.title, artists, release.tracks
            )
            .as_str(),
        )
        .append_pair("fmt", "json")
        .append_pair("limit", COUNT.to_string().as_str());
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

    let json: musicbrainz::ReleaseSearch =
        serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text.as_str()))
            .map_err(|e| {
                eyre!(
                    "Error while decoding JSON at path {}: {}",
                    e.path().to_string(),
                    e
                )
            })?;
    Ok(json.releases.into())
}

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(db)
            .await?
            .ok_or(eyre!("Import not found"))?;

        let combined_search_results = search(&import.source_release).await.map_err(|err| {
            eyre!(
                "Error while fetching MusicBrainz for album releases: {}",
                err
            )
        })?;
        let fetch_release_tasks = combined_search_results
            .releases
            .iter()
            .map(|rel| InsertTask {
                name: TaskName::ImportFetchRelease,
                payload: Some(json!(super::fetch_release::Data {
                    import_id: self.0,
                    release_id: rel.id,
                })),
                depends_on: Vec::new(),
                duration: Duration::seconds(60),
            })
            .collect::<Vec<_>>();

        let mut import_active = import.into_active_model();
        import_active.artists =
            ActiveValue::Set(entity::import::Artists(combined_search_results.artists));
        import_active.artist_credits = ActiveValue::Set(entity::import::ArtistCredits(
            combined_search_results.artist_credits,
        ));
        import_active.releases =
            ActiveValue::Set(entity::import::Releases(combined_search_results.releases));
        import_active.mediums =
            ActiveValue::Set(entity::import::Mediums(combined_search_results.mediums));
        import_active.tracks =
            ActiveValue::Set(entity::import::Tracks(combined_search_results.tracks));
        import_active.artist_track_relations = ActiveValue::Set(
            entity::import::ArtistTrackRelations(combined_search_results.artist_track_relations),
        );
        import_active.artist_credit_releases = ActiveValue::Set(
            entity::import::ArtistCreditReleases(combined_search_results.artist_credit_releases),
        );
        import_active.artist_credit_tracks = ActiveValue::Set(entity::import::ArtistCreditTracks(
            combined_search_results.artist_credit_tracks,
        ));

        import_active.update(&tx).await?;
        let tasks = push(&fetch_release_tasks).await?;
        // let rank_id = schedule_tasks(tx, import_job, vec![TaskData::RankRelease], &ids).await?;
        // let cover_ids = schedule_tasks(tx, import_job, vec![TaskData::FetchCover], &[rank_id]).await?;
        //  schedule_tasks(tx, import_job, vec![TaskData::RankCover], &ids).await?;

        // for result in compressed_search_results.into_iter() {
        //     search_results.push(fetch::get(result.0.release.id.to_string().as_str()).await?);
        // }
        // let mut rated_search_results = search_results
        //     .into_iter()
        //     .map(|search_result| {
        //         let rank::Rating(rating, mapping) = rank::rate_and_match(&tracks, &search_result);
        //         RatedSearchResult {
        //             rating,
        //             search_result,
        //             mapping,
        //         }
        //     })
        //     .collect::<Vec<_>>();
        // rated_search_results
        //     .sort_by(|a, b| a.rating.partial_cmp(&b.rating).unwrap_or(Ordering::Equal));
        // let fetch::SearchResult(full_release, _) = rated_search_results
        //     .first()
        //     .map(|r| r.search_result.clone())
        //     .ok_or(eyre!("No results found"))?;
        // let covers_by_provider = fetch::cover::search(library, &full_release).await?;
        // let covers = rank::rank_covers(library, covers_by_provider, &full_release);
        // let selected = (
        //     full_release.release.id,
        //     if !covers.is_empty() { Some(0) } else { None },
        // );
        Ok(())
    }
}
