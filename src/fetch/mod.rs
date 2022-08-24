mod musicbrainz;

use crate::album::{AlbumLike, ReleaseLike};
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
    album: Box<dyn AlbumLike>,
) -> Result<Vec<Box<dyn ReleaseLike>>> {
    let titles = album.title()?;
    let artists = album.artist()?;
    let tracks = album.tracks();
    let combinations = artists
        .iter()
        .map(|artist| {
            titles
                .iter()
                .map(|title| (artist, title))
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    for f in fetchers {
        for (artist, title) in combinations.clone() {
            result.append(
                &mut f
                    .search(artist.to_string(), title.to_string(), tracks.len())
                    .await?,
            );
        }
    }
    Ok(result)
}
