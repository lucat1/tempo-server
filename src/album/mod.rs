use super::track::{TrackFile, TrackLike};
use crate::util::dedup;
use eyre::Result;

#[derive(Clone, Debug)]
pub struct FileAlbum {
    pub tracks: Vec<TrackFile>,
}

impl FileAlbum {
    pub fn from_tracks(tracks: Vec<TrackFile>) -> Result<FileAlbum> {
        Ok(FileAlbum { tracks })
    }

    pub fn artists(&self) -> Result<Vec<String>> {
        let artists = self
            .tracks
            .iter()
            .map(|t| t.album_artists())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .flatten()
            .map(|s| s.clone())
            .collect::<Vec<_>>();
        Ok(dedup(artists))
    }

    pub fn titles(&self) -> Result<Vec<String>> {
        let titles = self
            .tracks
            .iter()
            .map(|t| t.album_title())
            .collect::<Result<Vec<_>>>()?;
        Ok(dedup(titles))
    }
}
