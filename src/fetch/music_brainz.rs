use super::Fetch;
use crate::album::AlbumLike;
use async_trait::async_trait;
use eyre::Result;
use std::collections::HashMap;

static DEFAULT_COUNT: u32 = 50;
static SEARCH_URL: &str =
    "http://musicbrainz.org/ws/2/release/?query=release:{} artist:{}&fmt=json";

pub struct MusicBrainz {
    key: String,
    count: u32,
}

impl MusicBrainz {
    pub fn new(key: Option<String>, count: Option<u32>) -> Self {
        MusicBrainz {
            key: key.or(Some(String::new())).unwrap(),
            count: count.or(Some(DEFAULT_COUNT)).unwrap(),
        }
    }
}

#[async_trait]
impl Fetch for MusicBrainz {
    async fn search(&self, artist: String, album_title: String) -> Result<Vec<Box<dyn AlbumLike>>> {
        let resp = reqwest::get("https://httpbin.org/ip")
            .await?
            .json::<HashMap<String, String>>()
            .await?;
        println!("{:#?}", resp);

        Ok(Vec::new())
    }
}
