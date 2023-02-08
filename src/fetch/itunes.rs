use serde_derive::{Deserialize, Serialize};
use setting::ArtProvider;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Itunes {
    pub results: Vec<ItunesResult>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItunesResult {
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "artworkUrl100")]
    pub artwork_url_100: String,
    pub max_size: Option<usize>,
}

impl From<Itunes> for Vec<super::cover::Cover> {
    fn from(caa: Itunes) -> Self {
        caa.results
            .into_iter()
            .filter_map(|i| {
                i.max_size.map(|s| super::cover::Cover {
                    provider: ArtProvider::Itunes,
                    urls: vec![i
                        .artwork_url_100
                        .replace("100x100", format!("{}x{}", s, s).as_str())],
                    width: s,
                    height: s,
                    title: i.collection_name,
                    artist: i.artist_name,
                })
            })
            .collect()
    }
}
