use eyre::{eyre, Result};
use sea_orm::{DbConn, EntityTrait};
use serde::Deserialize;

use crate::fetch::lastfm;
use base::setting::get_settings;
use entity::{full::ArtistInfo, user_connection::Named};

#[derive(Debug, Clone)]
pub struct Data {
    pub provider: entity::ConnectionProvider,
    pub username: String,
    pub time: time::OffsetDateTime,

    pub track: entity::Track,
    pub artist_credits: Vec<entity::ArtistCredit>,
    pub artists: Vec<entity::Artist>,
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

pub async fn run(db: &DbConn, data: Data) -> Result<()> {
    let settings = get_settings()?;
    let full_track = entity::full::FullTrack {
        track: data.track.clone(),
        artist_credit: data.artist_credits.clone(),
        artist: data.artists.clone(),
        artist_credit_track: Vec::new(),
        artist_track_relation: Vec::new(),
    };
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

            let connection_data: entity::user_connection::LastFMData =
                serde_json::from_value(connection.data.to_owned())?;
            let url = lastfm::LASTFM_BASE_URL.clone();
            let mut body: Vec<(String, String)> = vec![
                ("artist".to_string(), full_track.get_joined_artists()?),
                ("track".to_string(), data.track.title),
                (
                    "timestamp".to_string(),
                    data.time.unix_timestamp().to_string(),
                ),
                ("mbid".to_string(), data.track.id.to_string()),
                ("method".to_string(), "track.scrobble".to_string()),
                ("format".to_string(), "json".to_string()),
                ("api_key".to_string(), lastfm.apikey.to_owned()),
                ("sk".to_string(), connection_data.token),
            ];
            let signature = lastfm::signature(
                body.iter().map(|(k, v)| (k, v)),
                lastfm.shared_secret.as_str(),
            );
            body.push(("api_sig".to_string(), signature));
            tracing::trace! {?body,"Scrobbling to last.fm"};

            let req = common::fetch::CLIENT.post(url).form(&body).build()?;
            let res = lastfm::send_request(req).await?;
            let raw_data: LastFMScrobbleResponse = res.json().await.map_err(|e| eyre!(e))?;
            tracing::trace! {?raw_data, "Last.fm scrobble response"}

            match raw_data {
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
