mod music_brainz;

use crate::album::AlbumLike;
use async_trait::async_trait;
use eyre::Result;

use self::music_brainz::MusicBrainz;

#[async_trait]
pub trait Fetch {
    async fn search(&self, artist: String, album_title: String) -> Result<Vec<Box<dyn AlbumLike>>>;
}

pub fn default_fetchers() -> Vec<Box<dyn Fetch>> {
    vec![Box::new(MusicBrainz::new(None, None))]
}

pub async fn search(
    fetchers: Vec<Box<dyn Fetch>>,
    album: Box<dyn AlbumLike>,
) -> Result<Vec<Box<dyn AlbumLike>>> {
    let titles = album.title()?;
    let artists = album.artist()?;
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
            result.append(&mut f.search(artist.to_string(), title.to_string()).await?);
        }
    }
    Ok(result)
}
