pub mod cover;
pub mod cover_art_archive;
pub mod itunes;
pub mod structures;

use crate::models::{Artists, GroupTracks, UNKNOWN_ARTIST};
use const_format::formatcp;
pub use cover::Cover;
use eyre::{bail, eyre, Context, Result};
use lazy_static::lazy_static;
use log::trace;
use reqwest::header::USER_AGENT;
use std::sync::Arc;
use std::time::Instant;

static COUNT: u32 = 8;
static MB_USER_AGENT: &str =
    formatcp!("{}/{} ({})", crate::CLI_NAME, crate::VERSION, crate::GITHUB);
lazy_static! {
    pub static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

pub async fn search(
    release: &entity::Release,
    tracks: usize,
) -> Result<Vec<crate::models::Release>> {
    let start = Instant::now();
    let raw_artists = release.artists.joined();
    let artists = match raw_artists.as_str() {
        UNKNOWN_ARTIST => "",
        s => s,
    };
    let res = CLIENT
        .get(format!(
            "http://musicbrainz.org/ws/2/release/?query=release:{} artist:{} tracks:{}&fmt=json&limit={}",
            release.title, artists, tracks, COUNT
        ))
        .header(USER_AGENT, MB_USER_AGENT)
        .send()
        .await?;
    let req_time = start.elapsed();
    trace!("MusicBrainz HTTP request took {:?}", req_time);
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

    let json: ReleaseSearch =
        serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text.as_str()))
            .map_err(|e| eyre!("Error {} at path {}", e, e.path().to_string()))
            .wrap_err(eyre!("Error while decoding JSON: {}", text))?;
    let json_time = start.elapsed();
    trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
    Ok(json.releases.into_iter().map(|v| v.into()).collect())
}

pub async fn get(id: &str) -> Result<(crate::models::Release, Vec<crate::models::Track>)> {
    let start = Instant::now();
    let res = CLIENT
        .get(format!(
            "http://musicbrainz.org/ws/2/release/{}?fmt=json&inc={}",
            id,
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
                "recording-level-rels"
            ]
            .join("+")
        ))
        .header(USER_AGENT, MB_USER_AGENT)
        .send()
        .await?;
    let req_time = start.elapsed();
    trace!("MusicBrainz HTTP request took {:?}", req_time);
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

    let json: Arc<Release> =
        serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text.as_str()))
            .map_err(|e| eyre!("Error {} at path {}", e, e.path().to_string()))
            .wrap_err(eyre!("Error while decoding JSON: {}", text))?;
    let json_time = start.elapsed();
    trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
    json.group_tracks()
}
