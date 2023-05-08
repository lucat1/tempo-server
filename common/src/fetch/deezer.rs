use eyre::{bail, Result};
use serde_derive::{Deserialize, Serialize};
use std::time::Instant;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use super::{Cover, CLIENT};
use base::setting::ArtProvider;
use entity::full::{ArtistInfo, FullRelease};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Deezer {
    pub data: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Release {
    pub artist: Artist,
    pub album: Album,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Artist {
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Album {
    pub title: String,
    pub cover_small: String,
    pub cover: String,
    pub cover_medium: String,
    pub cover_big: String,
    pub cover_xl: String,
}

#[derive(EnumIter)]
enum Size {
    CoverSmall,
    Cover,
    CoverMedium,
    CoverBig,
    CoverXL,
}

fn get_image_of_type(album: &Album, size: &Size) -> (String, usize) {
    match size {
        Size::CoverSmall => (album.cover_small.to_string(), 56),
        Size::Cover => (album.cover.to_string(), 120),
        Size::CoverMedium => (album.cover_medium.to_string(), 250),
        Size::CoverBig => (album.cover_big.to_string(), 500),
        Size::CoverXL => (album.cover_xl.to_string(), 1000),
    }
}

pub async fn fetch(full_release: &FullRelease) -> Result<Vec<Cover>> {
    let FullRelease { release, .. } = full_release;
    let start = Instant::now();
    let res = CLIENT
        .get(format!(
            "https://api.deezer.com/search?order=RANKING&q=artist:\"{}\" album:\"{}\"",
            full_release.get_joined_artists()?,
            release.title.clone().as_str()
        ))
        .send()
        .await?;
    let req_time = start.elapsed();
    tracing::trace! {?req_time, "Time taken by the Deezer HTTP request"};
    if !res.status().is_success() {
        bail!(
            "Itunes request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let json = res.json::<Deezer>().await?;
    Ok(json
        .data
        .iter()
        .flat_map(|release| {
            Size::iter().map(|size_type| {
                let (url, size) = get_image_of_type(&release.album, &size_type);
                Cover {
                    provider: ArtProvider::Deezer,
                    url,
                    width: size,
                    height: size,
                    title: release.album.title.clone(),
                    artist: release.artist.name.clone(),
                }
            })
        })
        .collect())
}
