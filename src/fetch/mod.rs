mod musicbrainz;

use super::track::TrackLike;
use async_trait::async_trait;
use eyre::{eyre, Result};
use std::fmt::{Debug, Formatter, Result as FormatResult};

use self::musicbrainz::MusicBrainz;

pub trait ArtistLike: ArtistLikeBoxed {
    fn name(&self) -> String;
    fn mbid(&self) -> Option<String>;
    fn joinphrase(&self) -> Option<String>;
}

impl ArtistLike for String {
    fn name(&self) -> String {
        self.to_string()
    }
    fn mbid(&self) -> Option<String> {
        None
    }
    fn joinphrase(&self) -> Option<String> {
        Some("&".to_string())
    }
}

pub trait ArtistLikeBoxed: Send {
    fn clone_box(&self) -> Box<dyn ArtistLike>;
    fn fmt_box(&self, f: &mut Formatter<'_>) -> FormatResult;
}

impl<T> ArtistLikeBoxed for T
where
    T: 'static + ArtistLike + Clone + Debug,
{
    fn clone_box(&self) -> Box<dyn ArtistLike> {
        Box::new(self.clone())
    }

    fn fmt_box(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt(f)
    }
}

impl Clone for Box<dyn ArtistLike> {
    fn clone(&self) -> Box<dyn ArtistLike> {
        self.clone_box()
    }
}

impl Debug for Box<dyn ArtistLike> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt_box(f)
    }
}

pub trait ReleaseLike: ReleaseLikeBoxed {
    fn fetcher(&self) -> Option<Box<dyn Fetch>>;
    // Ether the mbid or any other id for a possible provider of this release's
    // metadata.
    fn id(&self) -> Option<String>;

    fn artists(&self) -> Vec<Box<dyn ArtistLike>>;
    fn title(&self) -> String;
    fn tracks(&self) -> Option<Vec<Box<dyn TrackLike>>>;
}

pub trait ReleaseLikeBoxed: Send {
    fn clone_box(&self) -> Box<dyn ReleaseLike>;
    fn fmt_box(&self, f: &mut Formatter<'_>) -> FormatResult;
}

impl<T> ReleaseLikeBoxed for T
where
    T: 'static + ReleaseLike + Clone + Debug,
{
    fn clone_box(&self) -> Box<dyn ReleaseLike> {
        Box::new(self.clone())
    }

    fn fmt_box(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt(f)
    }
}

impl Clone for Box<dyn ReleaseLike> {
    fn clone(&self) -> Box<dyn ReleaseLike> {
        self.clone_box()
    }
}

impl Debug for Box<dyn ReleaseLike> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt_box(f)
    }
}

#[async_trait]
pub trait Fetch {
    async fn search(&self, release: Box<dyn ReleaseLike>) -> Result<Vec<Box<dyn ReleaseLike>>>;
    async fn get(&self, release: Box<dyn ReleaseLike>) -> Result<Box<dyn ReleaseLike>>;
}

pub fn default_fetchers() -> Vec<Box<dyn Fetch>> {
    vec![Box::new(MusicBrainz::new(None, None))]
}

pub async fn search(
    fetchers: Vec<Box<dyn Fetch>>,
    release: Box<dyn ReleaseLike>,
) -> Result<Vec<Box<dyn ReleaseLike>>> {
    let mut result = Vec::new();
    for f in fetchers {
        result.append(&mut f.search(release.clone()).await?);
    }
    Ok(result)
}

pub async fn get(release: Box<dyn ReleaseLike>) -> Result<Box<dyn ReleaseLike>> {
    match release.fetcher() {
        Some(f) => f.get(release).await,
        None => Err(eyre!("The given release doesn't provide any fetcher")),
    }
}
