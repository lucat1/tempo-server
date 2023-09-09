use eyre::{eyre, Result};
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Method, Request,
};
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, ModelTrait, TransactionTrait};
use serde::{Deserialize, Serialize};
use taskie_client::{Task as TaskieTask, TaskKey};
use uuid::Uuid;

use crate::fetch::lastfm;
use crate::tasks::TaskName;
use base::setting::get_settings;
use entity::{
    full::{ArtistInfo, GetArtist, GetArtistCredits},
    user_connection::Named,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub provider: entity::ConnectionProvider,
    pub username: String,
    pub time: time::OffsetDateTime,

    pub track_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LastFMScrobbleResponse {
    Error(LastFMScrobbleResponseError),
    Success(serde_json::Value),
}

#[derive(Debug, Deserialize)]
struct LastFMScrobbleResponseError {
    code: usize,
    message: String,
}

struct TrackWithArtists(
    entity::Track,
    Vec<entity::ArtistCredit>,
    Vec<entity::Artist>,
);

impl GetArtistCredits for TrackWithArtists {
    fn get_artist_credits(&self) -> Vec<&entity::ArtistCredit> {
        self.1.iter().map(|ac| ac).collect()
    }
}

impl GetArtist for TrackWithArtists {
    fn get_artist(&self, id: Uuid) -> Option<&entity::Artist> {
        self.2.iter().find(|a| a.id == id)
    }
}

#[async_trait::async_trait]
impl super::TaskTrait for Data {
    async fn run<C>(&self, db: &C, _task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let settings = get_settings()?;
        let track = entity::TrackEntity::find_by_id(self.track_id)
            .one(&tx)
            .await?
            .ok_or(eyre!(
                "Track to be scrobbled doesn't exist: {}",
                self.track_id
            ))?;
        let artist_credits = track
            .find_related(entity::ArtistCreditEntity)
            .all(&tx)
            .await?;
        let artists: Vec<_> = artist_credits
            .load_one(entity::ArtistEntity, &tx)
            .await?
            .into_iter()
            .flatten()
            .collect();
        let track_with_artists = TrackWithArtists(track, artist_credits, artists);
        match self.provider {
            entity::ConnectionProvider::LastFM => {
                let lastfm = settings
                    .connections
                    .lastfm
                    .as_ref()
                    .ok_or(eyre!("Provider {} not configured", self.provider.name()))?;
                let connection = entity::UserConnectionEntity::find_by_id((
                    self.username.to_owned(),
                    self.provider,
                ))
                .one(&tx)
                .await?
                .ok_or(eyre!(
                    "Scrobbling user is not connected to the required service"
                ))?;

                let connection_self: entity::user_connection::LastFMData =
                    serde_json::from_value(connection.data.to_owned())?;
                let url = lastfm::LASTFM_BASE_URL.clone();
                let mut body: Vec<(String, String)> = vec![
                    (
                        "artist".to_string(),
                        track_with_artists.get_joined_artists()?,
                    ),
                    ("track".to_string(), track_with_artists.0.title.to_owned()),
                    (
                        "timestamp".to_string(),
                        self.time.unix_timestamp().to_string(),
                    ),
                    ("mbid".to_string(), self.track_id.to_string()),
                    ("method".to_string(), "track.scrobble".to_string()),
                    ("format".to_string(), "json".to_string()),
                    ("api_key".to_string(), lastfm.apikey.to_owned()),
                    ("sk".to_string(), connection_self.token),
                ];
                let signature = lastfm::signature(
                    body.iter().map(|(k, v)| (k, v)),
                    lastfm.shared_secret.as_str(),
                );
                body.push(("api_sig".to_string(), signature));
                tracing::trace! {?body,"Scrobbling to last.fm"};

                // taken from https://docs.rs/reqwest/latest/src/reqwest/async_impl/request.rs.html#406-424
                let body = serde_urlencoded::to_string(body)?;
                let mut req = Request::new(Method::POST, url);
                req.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("application/x-www-form-urlencoded"),
                );
                *req.body_mut() = Some(body.into());
                let res = lastfm::send_request(req).await?;
                let raw_self: LastFMScrobbleResponse = res.json().await.map_err(|e| eyre!(e))?;
                tracing::trace! {?raw_self, "Last.fm scrobble response"}

                match raw_self {
                    LastFMScrobbleResponse::Success(_) => Ok(()),
                    LastFMScrobbleResponse::Error(e) => Err(eyre!(
                        "Last.fm returned an error while scrobbling (code: {}): {}",
                        e.code,
                        e.message
                    )),
                }
            }
        }
    }
}
