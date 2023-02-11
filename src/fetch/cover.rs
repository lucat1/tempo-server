use entity::full::FullRelease;
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::{io::Reader as ImageReader, GenericImage, ImageOutputFormat, RgbaImage};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{trace, warn};
use mime::Mime;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use setting::{get_settings, ArtProvider, Settings};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Cursor;
use std::time::Instant;

use super::amazondigital;
use super::cover_art_archive::CoverArtArchive;
use super::itunes::Itunes;
use super::CLIENT;

lazy_static! {
    static ref HEADERS_FOR_PROVIDER: HashMap<ArtProvider, HeaderMap> =
        [(ArtProvider::AmazonDigital, amazondigital::HEADERS.clone()),]
            .iter()
            .cloned()
            .collect();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cover {
    pub provider: ArtProvider,
    pub urls: Vec<String>,
    pub width: usize,
    pub height: usize,
    pub title: String,
    pub artist: String,
}

// Covers are sorted by picture size
impl Ord for Cover {
    fn cmp(&self, other: &Self) -> Ordering {
        let s1 = self.width * self.height;
        let s2 = other.width * other.height;
        s1.cmp(&s2)
    }
}

impl PartialOrd for Cover {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Cover {
    fn eq(&self, other: &Self) -> bool {
        self.width * self.height == other.width * other.height
    }
}
impl Eq for Cover {}

static DEFAULT_COUNTRY: &str = "US";
static ITUNES_COUNTRIES: &[&str] = &[
    "AE", "AG", "AI", "AL", "AM", "AO", "AR", "AT", "AU", "AZ", "BB", "BE", "BF", "BG", "BH", "BJ",
    "BM", "BN", "BO", "BR", "BS", "BT", "BW", "BY", "BZ", "CA", "CG", "CH", "CL", "CN", "CO", "CR",
    "CV", "CY", "CZ", "DE", "DK", "DM", "DO", "DZ", "EC", "EE", "EG", "ES", "FI", "FJ", "FM", "FR",
    "GB", "GD", "GH", "GM", "GR", "GT", "GW", "GY", "HK", "HN", "HR", "HU", "ID", "IE", "IL", "IN",
    "IS", "IT", "JM", "JO", "JP", "KE", "KG", "KH", "KN", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LK", "LR", "LT", "LU", "LV", "MD", "MG", "MK", "ML", "MN", "MO", "MR", "MS", "MT", "MU", "MW",
    "MX", "MY", "MZ", "NA", "NE", "NG", "NI", "NL", "NP", "NO", "NZ", "OM", "PA", "PE", "PG", "PH",
    "PK", "PL", "PT", "PW", "PY", "QA", "RO", "RU", "SA", "SB", "SC", "SE", "SG", "SI", "SK", "SL",
    "SN", "SR", "ST", "SV", "SZ", "TC", "TD", "TH", "TJ", "TM", "TN", "TR", "TT", "TW", "TZ", "UA",
    "UG", "US", "UY", "UZ", "VC", "VE", "VG", "VN", "YE", "ZA", "ZW",
];

pub async fn probe(url: &str, option_headers: Option<HeaderMap>) -> bool {
    let mut req = CLIENT.head(url);
    if let Some(headers) = option_headers {
        req = req.headers(headers);
    }
    req.send()
        .await
        .ok()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn fetch_itunes(release: &FullRelease, _: &Settings) -> Result<Vec<Cover>> {
    let FullRelease {
        release, artist, ..
    } = release;
    let start = Instant::now();
    let raw_country = release
        .country
        .clone()
        .unwrap_or(DEFAULT_COUNTRY.to_string());
    let country = if ITUNES_COUNTRIES.contains(&raw_country.as_str()) {
        raw_country.as_str()
    } else {
        DEFAULT_COUNTRY
    };

    // TODO: make "," configurable
    let res = CLIENT
        .get(format!(
            "http://itunes.apple.com/search?media=music&entity=album&country={}&term={}",
            country,
            artist.into_iter().map(|a| a.name.clone()).join(",")
                + " "
                + release.title.clone().as_str()
        ))
        .send()
        .await?;
    let req_time = start.elapsed();
    trace!("Itunes HTTP request took {:?}", req_time);
    if !res.status().is_success() {
        bail!(
            "Itunes request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let mut json = res.json::<Itunes>().await?;
    for item in json.results.iter_mut() {
        for size in [5000, 1200, 600] {
            let url = item
                .artwork_url_100
                .replace("100x100", format!("{}x{}", size, size).as_str());
            if probe(url.as_str(), None).await {
                item.max_size = Some(size);
                break;
            } else {
                continue;
            }
        }
    }
    let json_time = start.elapsed();
    trace!("Itunes JSON parse took {:?}", json_time - req_time);
    Ok(json.into())
}

pub async fn fetch_cover_art_archive(
    full_release: &FullRelease,
    settings: &Settings,
) -> Result<Vec<Cover>> {
    let FullRelease {
        release, artist, ..
    } = full_release;
    let start = Instant::now();
    let res = CLIENT
        .get(format!(
            "http://coverartarchive.org/{}/{}",
            if settings.art.cover_art_archive_use_release_group {
                "release-group"
            } else {
                "release"
            },
            if settings.art.cover_art_archive_use_release_group {
                release.release_group_id.unwrap_or(release.id)
            } else {
                release.id
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
    // TODO: make the "," configurable
    Ok(json.into(
        release.title.clone(),
        artist.into_iter().map(|a| a.name.clone()).join(","),
    ))
}

pub async fn search_covers(release: &FullRelease) -> Result<Vec<Vec<Cover>>> {
    let settings = get_settings()?;
    let mut v = vec![];
    for provider in settings.art.providers.iter() {
        let res = match *provider {
            ArtProvider::CoverArtArchive => fetch_cover_art_archive(release, settings).await,
            ArtProvider::Itunes => fetch_itunes(release, settings).await,
            ArtProvider::AmazonDigital => amazondigital::fetch(release, settings).await,
        };
        match res {
            Ok(r) => v.push(r),
            Err(e) => warn!("Error while fetching image from {:?}: {}", provider, e),
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

pub async fn get_cover(cover: Cover) -> Result<(Vec<u8>, Mime)> {
    let settings = get_settings()?;
    let mut img = RgbaImage::new(cover.width as u32, cover.height as u32);
    let per_side = f64::sqrt(cover.urls.len() as f64) as usize;
    let (mut x, mut y, quadrant_size_x, quadrant_size_y) = (
        0,
        0,
        (cover.width / per_side) as u32,
        (cover.height / per_side) as u32,
    );
    for (i, url) in cover.urls.into_iter().enumerate() {
        let mut start = Instant::now();
        let mut req = CLIENT.get(url);
        if let Some(headers) = HEADERS_FOR_PROVIDER.get(&cover.provider) {
            trace!("Applying headers for the download: {:?}", headers);
            req = req.headers(headers.clone());
        }
        let res = req.send().await?;
        trace!("Download of cover art #{} took {:?}", i, start.elapsed());
        start = Instant::now();
        if !res.status().is_success() {
            bail!(
                "Fetch request for cover art returned non-success error code: {} {}",
                res.status(),
                res.text().await?
            );
        }
        let bytes = res.bytes().await?;
        img.copy_from(
            &ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()?
                .decode()?,
            x,
            y,
        )?;
        trace!("Parse of cover art #{} took {:?}", i, start.elapsed());
        x += quadrant_size_x;
        if i % per_side == 0 {
            x = 0;
            y += quadrant_size_y;
        }
    }
    let start = Instant::now();
    let resized = if settings.art.width < img.width() || settings.art.height < img.height() {
        let converted = resize(
            &img,
            settings.art.width,
            settings.art.height,
            FilterType::Gaussian,
        );
        trace!(
            "Conversion of cover art took {:?} (from {}x{} to {}x{})",
            start.elapsed(),
            img.width(),
            img.height(),
            converted.width(),
            converted.height()
        );
        converted
    } else {
        img
    };
    let mut bytes: Vec<u8> = Vec::new();
    let format: ImageOutputFormat = settings.art.format.clone().into();
    resized.write_to(&mut Cursor::new(&mut bytes), format)?;
    Ok((bytes, settings.art.format.mime()))
}
