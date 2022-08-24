mod musicbrainz;

use crate::album::{FileAlbum, ReleaseLike};
use async_trait::async_trait;
use eyre::Result;

use self::musicbrainz::MusicBrainz;

#[async_trait]
pub trait Fetch {
    async fn search(
        &self,
        artist: String,
        album_title: String,
        track_count: usize,
    ) -> Result<Vec<Box<dyn ReleaseLike>>>;
}

pub fn default_fetchers() -> Vec<Box<dyn Fetch>> {
    vec![Box::new(MusicBrainz::new(None, None))]
}

pub async fn search(
    fetchers: Vec<Box<dyn Fetch>>,
    artist: String,
    title: String,
    track_count: usize,
) -> Result<Vec<Box<dyn ReleaseLike>>> {
    let mut result = Vec::new();
    for f in fetchers {
        result.append(&mut f.search(artist.clone(), title.clone(), track_count).await?);
    }
    Ok(result)
}
