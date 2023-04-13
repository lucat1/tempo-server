mod cover_art_archive;
mod deezer;
mod itunes;
mod music_brainz;

pub mod cover;
use base::setting::Library;
pub use cover::Cover;
pub use music_brainz::ReleaseSearch;

use crate::internal::{Release, UNKNOWN_ARTIST};
use const_format::formatcp;
use eyre::{bail, eyre, Context, Result};
use lazy_static::lazy_static;
use log::trace;
use reqwest::header::USER_AGENT;
use serde::Serialize;
use std::time::Instant;

use self::music_brainz::TrackWithMediumId;

static COUNT: u32 = 8;
static MB_USER_AGENT: &str = formatcp!("{}/{} ({})", base::CLI_NAME, base::VERSION, base::GITHUB);
lazy_static! {
    pub static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

#[derive(Serialize, Clone, Debug)]
pub struct SearchResult(
    pub entity::full::FullRelease,
    pub Vec<entity::full::FullTrack>,
);

fn release_to_result(library: &Library, r: music_brainz::Release) -> Result<SearchResult> {
    let release: entity::full::FullRelease = r.clone().into_full_release(library)?;
    let tracks: Vec<entity::full::FullTrack> = r
        .media
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter_map(|(i, m)| {
            m.tracks.map(|tracks| {
                tracks
                    .into_iter()
                    .map(|t| music_brainz::TrackWithMediumId(t, release.medium[i].id))
                    .collect::<Vec<TrackWithMediumId>>()
            })
        })
        .flatten()
        .map(|twm: TrackWithMediumId| twm.into())
        .collect();
    Ok(SearchResult(release, tracks))
}

pub async fn search(library: &Library, release: &Release) -> Result<Vec<SearchResult>> {
    let start = Instant::now();
    let raw_artists = release.artists.join(", ");
    let artists = match raw_artists.as_str() {
        UNKNOWN_ARTIST => "",
        s => s,
    };
    // TODO: don't apply the tracks filter when it's None
    let res = CLIENT
        .get(format!(
            "http://musicbrainz.org/ws/2/release/?query=release:{} artist:{} tracks:{}&fmt=json&limit={}",
            release.title, artists, release.tracks.unwrap_or_default(), COUNT
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
    json.releases
        .into_iter()
        .map(|r| release_to_result(library, r))
        .collect()
}

pub async fn get(library: &Library, id: &str) -> Result<SearchResult> {
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

    let json: music_brainz::Release =
        serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text.as_str()))
            .map_err(|e| eyre!("Error {} at path {}", e, e.path().to_string()))
            .wrap_err(eyre!("Error while decoding JSON: {}", text))?;
    let json_time = start.elapsed();
    trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
    release_to_result(library, json)
}
