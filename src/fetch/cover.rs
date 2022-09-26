use crate::fetch::structures::Itunes;
use crate::models::Artists;
use crate::settings::ArtProvider;
use crate::{Settings, SETTINGS};
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::ImageOutputFormat;
use image::{io::Reader as ImageReader, DynamicImage};
use log::{debug, trace};
use mime::Mime;
use std::io::Cursor;
use std::time::Instant;

use super::structures::Cover;
use super::structures::CoverArtArchive;
use super::CLIENT;

static DEFAULT_COUNTRY: &str = "US";

pub async fn fetch_itunes(release: &crate::models::Release, _: &Settings) -> Result<Vec<Cover>> {
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
    Ok(json.into())
}

pub async fn fetch_cover_art_archive(
    release: &crate::models::Release,
    settings: &Settings,
) -> Result<Vec<Cover>> {
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
    Ok(json.into(release.title.clone(), release.artists.joined()))
}

pub async fn search_covers(release: &crate::models::Release) -> Result<Vec<Vec<Cover>>> {
    let settings = SETTINGS.get().ok_or(eyre!("Could not read settings"))?;
    let mut v = vec![];
    for provider in settings.art.providers.iter() {
        let res = match *provider {
            ArtProvider::CoverArtArchive => fetch_cover_art_archive(release, settings).await,
            ArtProvider::Itunes => fetch_itunes(release, settings).await,
        };
        match res {
            Ok(r) => v.push(r),
            Err(e) => debug!("Error while fetching image from {:?}: {}", provider, e),
        }
    }
    if v.is_empty() {
        Err(eyre!(
            "No cover art found in all providers: {:?}",
            settings.art.providers
        ))
    } else {
        Ok(v)
    }
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
