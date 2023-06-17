use eyre::{eyre, Result};
use reqwest::{Method, Request};
use sea_orm::{DbConn, EntityTrait, LoaderTrait};
use serde::{Serialize,Deserialize};
use uuid::Uuid;

use crate::fetch::lastfm;
use base::setting::get_settings;
use entity::{full::ArtistInfo, user_connection::Named};

#[derive(Debug, Clone)]
pub struct Data {
    provider: entity::ConnectionProvider,
    username: String,
    track: entity::full::FullTrack,
    time: time::OffsetDateTime,
}

#[derive(Debug,Serialize)]
struct ScrobbleRequest {
    artist: Vec<String>,
    track: Vec<String>,
    timestamp: Vec<i64>,
    mbid: Vec<Uuid>
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LastFMScrobbleResponse {
    Success(LastFMScrobbleResponseSuccess),
    Error(LastFMScrobbleResponseError),
}

#[derive(Debug, Deserialize)]
struct LastFMScrobbleResponseSuccess {
    lfm: LFM,
}

#[derive(Debug, Deserialize)]
struct LFM {
    scrobbles: Vec<Scrobble>,
}

#[derive(Debug, Deserialize)]
struct Scrobble {
    track: String,
    artist: String,
    album: String,
    album_artist: String,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct LastFMScrobbleResponseError {
    code: usize,
    message: String,
}

pub async fn run(db: &DbConn, data: Data) -> Result<()> {
    let settings = get_settings()?;
    match data.provider {
        entity::ConnectionProvider::LastFM => {
            let lastfm = settings
                .connections
                .lastfm
                .as_ref()
                .ok_or(eyre!("Provider {} not configured", data.provider.name()))?;
            let connection =
                entity::UserConnectionEntity::find_by_id((data.username, data.provider))
                    .one(db)
                    .await?
                    .ok_or(eyre!(
                        "Scrobbling user is not connected to the required service"
                    ))?;

            let scrobble = ScrobbleRequest{
                artist: vec![data.track.get_joined_artists()?],
                track: vec![data.track.track.title],
                timestamp: vec![data.time.unix_timestamp()],
                mbid: vec![data.track.track.id],
            };
            tracing::trace!{?scrobble,"Scrobbling to last.fm"};

            let connection_data: entity::user_connection::LastFMData =
                serde_json::from_value(connection.data.to_owned())?;
            let mut url = lastfm::LASTFM_BASE_URL.clone();
            url.query_pairs_mut()
                .append_pair("method", "track.scrobble")
                .append_pair("format", "json")
                .append_pair("api_key", lastfm.apikey.as_str())
                .append_pair("sk", connection_data.token.as_str());
            let signature = lastfm::signature(&url, lastfm.shared_secret.as_str());
            url.query_pairs_mut()
                .append_pair("api_sig", signature.as_str());
            let body = serde_json::to_string(&scrobble)?;
            let mut req = Request::new(Method::POST, url);
            let req_body = req.body_mut();
            *req_body = Some(body.into());
            let res = lastfm::send_request(req).await?;
            let raw_data: LastFMScrobbleResponse = res.json().await.map_err(|e| eyre!(e))?;
            tracing::trace!{?raw_data, "Last.fm scrobble response"}

            match raw_data {
                LastFMScrobbleResponse::Success(d) =>
            if d.lfm.scrobbles.len() != 1{
                Err(eyre!("Something went wrong, last.fm didn't report a single scrobble"))
            } else { Ok(()) },
                    LastFMScrobbleResponse::Error(e) => {
                Err(eyre!("Last.fm returned an error while scrobbling (code: {}): {}", e.code, e.message))
                }
            }
        }
    }
}
