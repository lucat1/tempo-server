mod structures;

use super::Fetch;
use crate::album::ReleaseLike;
use async_trait::async_trait;
use const_format::formatcp;
use eyre::{bail, Result};
use log::trace;
use reqwest::header::USER_AGENT;
use std::time::Instant;
use structures::ReleaseSearch;

static DEFAULT_COUNT: u32 = 50;
static MB_USER_AGENT: &str =
    formatcp!("{}/{} ({})", crate::CLI_NAME, crate::VERSION, crate::GITHUB);

pub struct MusicBrainz {
    count: u32,
    client: reqwest::Client,
}

impl MusicBrainz {
    pub fn new(_: Option<String>, count: Option<u32>) -> Self {
        MusicBrainz {
            count: count.or(Some(DEFAULT_COUNT)).unwrap(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Fetch for MusicBrainz {
    async fn search(
        &self,
        artist: String,
        album_title: String,
        track_count: usize,
    ) -> Result<Vec<Box<dyn ReleaseLike>>> {
        let start = Instant::now();
        let res = self
            .client
            .get(format!(
                "http://musicbrainz.org/ws/2/release/?query=release:{} artist:{}&fmt=json&limit={}",
                artist, album_title, self.count
            ))
            .header(USER_AGENT, MB_USER_AGENT)
            .send()
            .await?;
        let req_time = start.elapsed();
        trace!("MusicBrainz HTTP request took {:?}", req_time);
        if !res.status().is_success() {
            bail!(
                "Musicbrainz request returned non-success error code: {} {}",
                res.status(),
                res.text().await?
            );
        }
        let json = res.json::<ReleaseSearch>().await?;
        let json_time = start.elapsed();
        trace!("MusicBrainz JSON parse took {:?}", json_time - req_time);
        Ok(json
            .releases
            .iter()
            .map(|v| Box::new(v.clone()) as Box<dyn ReleaseLike>)
            .collect())
    }
}
