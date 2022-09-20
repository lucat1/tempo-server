mod structures;

use crate::fetch::structures::Itunes;
use crate::models::{Artists, GroupTracks, UNKNOWN_ARTIST};
use crate::settings::ArtProvider;
use crate::{Settings, SETTINGS};
use const_format::formatcp;
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::ImageOutputFormat;
use image::{io::Reader as ImageReader, DynamicImage};
use lazy_static::lazy_static;
use log::{debug, trace};
use mime::Mime;
use reqwest::header::USER_AGENT;
use std::io::Cursor;
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

static DEFAULT_COUNTRY: &str = "US";

pub async fn fetch_itunes(release: &crate::models::Release, _: &Settings) -> Result<String> {
    let start = Instant::now();
    let res = CLIENT
        .get(format!(
            "http://itunes.apple.com/search?media=music&entity=album&country={}&term={}",
            release
                .country
                .clone()
                .unwrap_or(DEFAULT_COUNTRY.to_string()),
            release.artists.joined() + " " + release.title.as_str()
        ))
        .send()
        .await?;
    let req_time = start.elapsed();
    trace!("Itunes HTTP request took {:?}", req_time);
    if !res.status().is_success() {
        bail!(
            "CoverArtArchive request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let json = res.json::<Itunes>().await?;
    let json_time = start.elapsed();
    trace!("Itunes JSON parse took {:?}", json_time - req_time);
    // TODO: sort to get better results, really important
    json.try_into()
}

pub async fn fetch_cover_art_archive(
    release: &crate::models::Release,
    settings: &Settings,
) -> Result<String> {
    let start = Instant::now();
    let res = CLIENT
        .get(format!(
            "http://coverartarchive.org/{}/{}",
            if settings.tagging.use_release_group {
                "release-group"
            } else {
                "release"
            },
            if settings.tagging.use_release_group {
                release.release_group_mbid.clone().ok_or(eyre!(
                    "The given release doesn't have an associated MusicBrainz relese-group id"
                ))?
            } else {
                release.mbid.clone().ok_or(eyre!(
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
    json.try_into()
}

pub async fn search_cover(release: &crate::models::Release) -> Result<(String, ArtProvider)> {
    let settings = SETTINGS.get().ok_or(eyre!("Could not read settings"))?;
    for provider in settings.art.providers.iter() {
        let res = match provider {
            ArtProvider::CoverArtArchive => fetch_cover_art_archive(release, settings).await,
            ArtProvider::Itunes => fetch_itunes(release, settings).await,
        };
        match res {
            Ok(v) => return Ok((v, provider.clone())),
            Err(e) => debug!("Error while fetching image from {:?}: {}", provider, e),
        }
    }
    return Err(eyre!(
        "No cover art found in all providers: {:?}",
        settings.art.providers
    ));
}

pub async fn get_cover(url: String) -> Result<(Vec<u8>, Mime)> {
    let start = Instant::now();
    let settings = SETTINGS.get().ok_or(eyre!("Could not read settings"))?;
    let res = CLIENT.get(url).send().await?;
    let req_time = start.elapsed();
    trace!("Fetch request for cover art took {:?}", req_time);
    if !res.status().is_success() {
        bail!(
            "Fetch request for cover art returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let bytes = res.bytes().await?;
    let bytes_time = start.elapsed();
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;
    // .map_err(|e| eyre!(e))?;
    trace!("Parse of cover art took {:?}", bytes_time - req_time);
    let resized = if settings.art.width < img.width() || settings.art.height < img.height() {
        let converted = resize(
            &img,
            settings.art.width,
            settings.art.height,
            FilterType::Gaussian,
        );
        let convert_time = start.elapsed();
        trace!(
            "Conversion of cover art took {:?} (from {}x{} to {}x{})",
            convert_time - bytes_time - req_time,
            img.width(),
            img.height(),
            converted.width(),
            converted.height()
        );
        DynamicImage::ImageRgba8(converted)
    } else {
        img
    };
    let mut bytes: Vec<u8> = Vec::new();
    let format: ImageOutputFormat = settings.art.format.clone().into();
    resized.write_to(&mut Cursor::new(&mut bytes), format)?;
    Ok((bytes, settings.art.format.mime()))
}
