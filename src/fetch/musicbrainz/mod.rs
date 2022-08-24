mod structures;

use super::Fetch;
use crate::album::ReleaseLike;
use async_trait::async_trait;
use const_format::formatcp;
use eyre::{eyre, Result};
use reqwest::header::USER_AGENT;
use std::cmp::Ordering;
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
        let mut res = self
            .client
            .get(format!(
                "http://musicbrainz.org/ws/2/release/?query=release:{} artist:{}&fmt=json&limit={}",
                artist, album_title, self.count
            ))
            .header(USER_AGENT, MB_USER_AGENT)
            .send()
            .await?
            .json::<ReleaseSearch>()
            .await?;
        res.releases.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .ok_or("Could not sort releases")
                .unwrap()
        });
        res.releases.sort_by(|a, b| match a.track_count {
            track_count => Ordering::Equal,
            _ => Ordering::Less,
        });
        Ok(res.releases.iter().map(|v| Box::new(v)).collect())
    }
}
