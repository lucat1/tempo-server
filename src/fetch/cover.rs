use crate::settings::{get_settings, ArtProvider, Settings};
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::ImageOutputFormat;
use image::{io::Reader as ImageReader, DynamicImage};
use log::{debug, trace};
use mime::Mime;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Cursor;
use std::time::Instant;

use super::cover_art_archive::CoverArtArchive;
use super::itunes::Itunes;
use super::CLIENT;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cover {
    pub provider: ArtProvider,
    pub url: String,
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

pub async fn probe(url: String) -> Option<()> {
    CLIENT.head(url).send().await.ok().map(|_| ())
}

pub async fn fetch_itunes(release: &entity::Release, _: &Settings) -> Result<Vec<Cover>> {
    let start = Instant::now();
    let raw_country = release.country.as_deref().unwrap_or(DEFAULT_COUNTRY);
    let country = if ITUNES_COUNTRIES.contains(&raw_country) {
        raw_country
    } else {
        DEFAULT_COUNTRY
    };

    let res = CLIENT
        .get(format!(
            "http://itunes.apple.com/search?media=music&entity=album&country={}&term={}",
            country,
            release.artists.joined() + " " + release.title.as_str()
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
            match probe(url).await {
                Some(_) => {
                    item.max_size = Some(size);
                    break;
                }
                None => continue,
            }
        }
    }
    let json_time = start.elapsed();
    trace!("Itunes JSON parse took {:?}", json_time - req_time);
    Ok(json.into())
}

pub async fn fetch_cover_art_archive(
    release: &entity::Release,
    settings: &Settings,
) -> Result<Vec<Cover>> {
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

pub async fn search_covers(release: &entity::Release) -> Result<Vec<Vec<Cover>>> {
    let settings = get_settings()?;
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
    let settings = get_settings()?;
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
