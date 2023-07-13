use base::setting::{ArtProvider, Library};
use entity::full::FullRelease;
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::DynamicImage;
use image::{io::Reader as ImageReader, ImageOutputFormat};
use mime::Mime;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Cursor;
use std::time::Instant;

use super::CLIENT;
use super::{cover_art_archive, deezer, itunes};

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
            Err(error) => {
                tracing::warn! {%provider, %error, "Error while fetching image from provider"}
            }
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

pub async fn get_cover(library: &Library, cover: &Cover) -> Result<(Vec<u8>, (u32, u32), Mime)> {
    let start = Instant::now();
    let res = CLIENT.get(cover.url.clone()).send().await?;
    let req_time = start.elapsed();
    tracing::trace! {?req_time, "Fetch request for cover art took"};
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
    tracing::trace! {prase_time = ?(bytes_time - req_time), "Parse of cover art took"};
    let resized = if library.art.width < img.width() || library.art.height < img.height() {
        let converted = resize(
            &img,
            library.art.width,
            library.art.height,
            FilterType::Gaussian,
        );
        let convert_time = start.elapsed();
        tracing::trace! {
            convert_time = ?(convert_time - bytes_time - req_time),
            src_width = img.width(),
            src_height = img.height(),
            dst_width = converted.width(),
            dst_height = converted.height(),
            "Done scaling/converting image",
        };
        DynamicImage::ImageRgba8(converted)
    } else {
        img
    };
    let mut bytes: Vec<u8> = Vec::new();
    let format: ImageOutputFormat = library.art.format.into();
    resized.write_to(&mut Cursor::new(&mut bytes), format)?;
    Ok((
        bytes,
        (resized.width(), resized.height()),
        library.art.format.mime(),
    ))
}
