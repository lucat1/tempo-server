mod musicbrainz;

use self::musicbrainz::MusicBrainz;
use crate::models::Release;
use async_trait::async_trait;
use eyre::{eyre, Result};
use std::fmt::Debug;

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";

#[async_trait]
pub trait Fetch: Debug {
    async fn search(&self, release: Release) -> Result<Vec<Release>>;
    async fn get(&self, release: Release) -> Result<Release>;
}

pub fn default_fetchers() -> Vec<Box<dyn Fetch>> {
    vec![Box::new(MusicBrainz::new(None, None))]
}

pub async fn search(
    fetchers: Vec<Box<dyn Fetch>>,
    release: Release,
) -> Result<Vec<Release>> {
    let mut result = Vec::new();
    for f in fetchers {
        result.append(&mut f.search(release.clone()).await?);
    }
    Ok(result)
}

pub async fn get(release: Release) -> Result<Release> {
    match release.fetcher.clone() {
        Some(f) => f.get(release).await,
        None => Err(eyre!("The given release doesn't provide any fetcher")),
    }
}
