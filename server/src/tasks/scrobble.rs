use eyre::{eyre, Result};
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Method, Request,
};
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, ModelTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fetch::lastfm;
use base::setting::get_settings;
use entity::{full::ArtistInfo, user_connection::Named};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
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

#[async_trait::async_trait]
impl super::TaskTrait for Task {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait,
    {
        let settings = get_settings()?;
        let track = entity::TrackEntity::find_by_id(self.track_id)
            .one(db)
            .await?
            .ok_or(eyre!(
                "Track to be scrobbled doesn't exist: {}",
                self.track_id
            ))?;
        let artist_credits = track
            .find_related(entity::ArtistCreditEntity)
            .all(db)
            .await?;
        let artists: Vec<_> = artist_credits
            .load_one(entity::ArtistEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();
        let full_track = entity::full::FullTrack {
            track,
            artist_credit: artist_credits,
            artist: artists,
            artist_credit_track: Vec::new(),
            artist_track_relation: Vec::new(),
        };
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
                .one(db)
                .await?
                .ok_or(eyre!(
                    "Scrobbling user is not connected to the required service"
                ))?;

                let connection_self: entity::user_connection::LastFMData =
                    serde_json::from_value(connection.data.to_owned())?;
                let url = lastfm::LASTFM_BASE_URL.clone();
                let mut body: Vec<(String, String)> = vec![
                    ("artist".to_string(), full_track.get_joined_artists()?),
                    ("track".to_string(), full_track.track.title.to_owned()),
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
