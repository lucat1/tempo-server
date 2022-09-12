mod structures;

use crate::models::{Artists, GroupTracks, UNKNOWN_ARTIST};
use const_format::formatcp;
use eyre::{bail, eyre, Result};
use lazy_static::lazy_static;
use log::trace;
use reqwest::header::USER_AGENT;
use std::sync::Arc;
use std::time::Instant;
use structures::{Release, ReleaseSearch};

static COUNT: u32 = 8;
static MB_USER_AGENT: &str =
    formatcp!("{}/{} ({})", crate::CLI_NAME, crate::VERSION, crate::GITHUB);
lazy_static! {
    pub static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

pub async fn search(
    release: &crate::models::Release,
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
    let json = res.json::<ReleaseSearch>().await?;
    let json_time = start.elapsed();
    trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
    Ok(json.releases.iter().map(|v| v.clone().into()).collect())
}

pub async fn get(
    release: &crate::models::Release,
) -> Result<(crate::models::Release, Vec<crate::models::Track>)> {
    let start = Instant::now();
    let id = release.mbid.clone().ok_or(eyre!(
        "The given release doesn't have an ID associated with it, can not fetch specific metadata"
    ))?;
    let res = CLIENT
        .get(format!(
            "http://musicbrainz.org/ws/2/release/{}?fmt=json&inc=artists+labels+recordings+genres",
            id
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
    let json = res.json::<Arc<Release>>().await?;
    let json_time = start.elapsed();
    trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
    json.group_tracks()
}
