use eyre::{bail, Result};
use reqwest::{Method, Request};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

use base::setting::{ArtProvider, Settings};
use entity::full::ArtistInfo;

use crate::fetch::musicbrainz;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverArtArchive {
    pub images: Vec<Image>,
}

impl CoverArtArchive {
    pub fn covers(self, title: String, artist: String) -> Vec<entity::import::Cover> {
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
                        sizes.get(size).map(|url| entity::import::Cover {
                            provider: ArtProvider::CoverArtArchive,
                            url: url.to_string(),
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

pub async fn search(
    settings: &Settings,
    full_release: &entity::full::FullRelease,
) -> Result<Vec<entity::import::Cover>> {
    let release = full_release.get_release();
    let (kind, value) = if settings.library.art.cover_art_archive_use_release_group {
        (
            "release-group",
            release.release_group_id.unwrap_or(release.id),
        )
    } else {
        ("release", release.id)
    };
    let res = musicbrainz::send_request(Request::new(
        Method::GET,
        format!("http://coverartarchive.org/{}/{}", kind, value).parse()?,
    ))
    .await?;

    if !res.status().is_success() {
        bail!(
            "CoverArtArchive request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let json = res.json::<CoverArtArchive>().await?;
    Ok(json.covers(release.title.clone(), full_release.get_joined_artists()?))
}
