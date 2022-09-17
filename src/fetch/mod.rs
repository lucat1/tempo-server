mod structures;

use crate::{
    models::{Artists, GroupTracks, UNKNOWN_ARTIST},
    SETTINGS,
};
use const_format::formatcp;
use eyre::{bail, eyre, Result};
use lazy_static::lazy_static;
use log::trace;
use reqwest::header::USER_AGENT;
use std::sync::Arc;
use std::time::Instant;
use structures::{Release, ReleaseSearch};

use self::structures::CoverArtArchive;

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
    Ok(json.releases.into_iter().map(|v| v.into()).collect())
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
    let json = res.json::<Arc<Release>>().await?;
    let json_time = start.elapsed();
    trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
    json.group_tracks()
}

pub async fn covers(release: &crate::models::Release) -> Result<Vec<Cover>> {
    let start = Instant::now();
    let raw_artists = release.artists.joined();
    let use_release_group = SETTINGS.get()?.tagging.use_release_group;
    let res = CLIENT
        .get(format!(
            "http://coverartarchive.org/{}/{}",
            if use_release_group {
                "release-group"
            } else {
                "release"
            },
            if use_release_group {
                release.release_group_mbid.ok_or(eyre!(
                    "The given release doesn't have an associated MusicBrainz relese-group id"
                ))?
            } else {
                release.mbid.ok_or(eyre!(
                    "The given release doesn't have an associated MusicBrainz id"
                ))?
            }
        ))
        .header(USER_AGENT, MB_USER_AGENT)
        .send()
        .await?;
    let req_time = start.elapsed();
    trace!("CoverArtArchive HTTP request took {:?}", req_time);
    if !res.status().is_success() {
        bail!(
            "CoverArtArchive request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let json = res.json::<CoverArtArchive>().await?;
    let json_time = start.elapsed();
    trace!("CoverArtArchive JSON parse took {:?}", json_time - req_time);
    Ok(json.images.into_iter().map(|v| v.into()).collect())
}
