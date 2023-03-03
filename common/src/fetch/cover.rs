use base::setting::{ArtProvider, Library};
use entity::full::FullRelease;
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::{io::Reader as ImageReader, GenericImage, ImageOutputFormat, RgbaImage};
use log::{trace, warn};
use mime::Mime;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Cursor;
use std::time::Instant;

use super::CLIENT;
use super::{cover_art_archive, deezer, itunes};

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

pub async fn search(library: &Library, release: &FullRelease) -> Result<Vec<Vec<Cover>>> {
    let mut v = vec![];
    for provider in library.art.providers.iter() {
        let res = match *provider {
            ArtProvider::CoverArtArchive => cover_art_archive::fetch(library, release).await,
            ArtProvider::Itunes => itunes::fetch(release).await,
            ArtProvider::Deezer => deezer::fetch(release).await,
        };
        match res {
            Ok(r) => v.push(r),
            Err(e) => warn!("Error while fetching image from {:?}: {}", provider, e),
        }
    }
    if v.is_empty() {
        Err(eyre!(
            "No cover art found in all providers: {:?}",
            library.art.providers
        ))
    } else {
        Ok(v)
    }
}

pub async fn get_cover(library: &Library, cover: Cover) -> Result<(Vec<u8>, Mime)> {
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
        let res = CLIENT.get(url).send().await?;
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
    let resized = if library.art.width < img.width() || library.art.height < img.height() {
        let converted = resize(
            &img,
            library.art.width,
            library.art.height,
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
    let format: ImageOutputFormat = library.art.format.clone().into();
    resized.write_to(&mut Cursor::new(&mut bytes), format)?;
    Ok((bytes, library.art.format.mime()))
}
