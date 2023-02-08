use serde_derive::{Deserialize, Serialize};
use setting::ArtProvider;
use std::collections::HashMap;

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
