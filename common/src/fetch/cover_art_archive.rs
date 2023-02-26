use eyre::{bail, Result};
use itertools::Itertools;
use log::trace;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use super::{Cover, CLIENT};
use base::setting::{get_settings, ArtProvider};
use entity::full::FullRelease;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverArtArchive {
    pub images: Vec<Image>,
}

impl CoverArtArchive {
    pub fn into(self, title: String, artist: String) -> Vec<super::cover::Cover> {
        self.images
            .into_iter()
            .filter_map(|i| {
                if i.front {
                    let sizes: HashMap<usize, String> = i
                        .thumbnails
                        .into_iter()
                        .filter_map(|(k, v)| k.parse::<usize>().ok().map(|d| (d, v)))
                        .collect();
                    sizes.keys().max().and_then(|size| {
                        sizes.get(size).map(|url| super::cover::Cover {
                            provider: ArtProvider::CoverArtArchive,
                            urls: vec![url.to_string()],
                            width: *size,
                            height: *size,
                            title: title.clone(),
                            artist: artist.clone(),
                        })
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    approved: bool,
    front: bool,
    thumbnails: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbnails {
    #[serde(rename = "250")]
    the_250: String,
    #[serde(rename = "500")]
    the_500: String,
    #[serde(rename = "1200")]
    the_1200: String,
    large: String,
    small: String,
}

pub async fn fetch(full_release: &FullRelease) -> Result<Vec<Cover>> {
    let settings = get_settings()?;
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
