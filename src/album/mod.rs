use super::track::{TrackFile, TrackLike};
use super::util::dedup;
use eyre::Result;
use std::fmt::{Debug, Formatter, Result as FormatResult};

#[derive(Clone, Debug)]
pub struct RoughAlbum {
    tracks: Vec<TrackFile>,
}

impl RoughAlbum {
    pub fn from_tracks(tracks: Vec<TrackFile>) -> Result<RoughAlbum> {
        Ok(RoughAlbum { tracks })
    }
}

pub trait AlbumLike: AlbumLikeBoxed {
    fn artist(&self) -> Result<Vec<String>>;
    fn title(&self) -> Result<Vec<String>>;
    fn tracks(&self) -> Vec<Box<dyn TrackLike>>;
}

impl AlbumLike for RoughAlbum {
    // Having a vector with length > 1 means there isn't consensus on:
    // - the album artist
    // - the album title
    // amongst the tracks.

    fn artist(&self) -> Result<Vec<String>> {
        Ok(dedup(
            self.tracks
                .iter()
                .map(|t| t.album_artist())
                .collect::<Result<Vec<_>>>()?,
        ))
    }

    fn title(&self) -> Result<Vec<String>> {
        Ok(dedup(
            self.tracks
                .iter()
                .map(|t| t.album_title())
                .collect::<Result<Vec<_>>>()?,
        ))
    }

    fn tracks(&self) -> Vec<Box<dyn TrackLike>> {
        self.tracks
            .iter()
            .map(|t| Box::new(t.clone()) as Box<dyn TrackLike>)
            .collect::<Vec<_>>()
    }
}

pub trait AlbumLikeBoxed {
    fn clone_box(&self) -> Box<dyn AlbumLike>;
    fn fmt_box(&self, f: &mut Formatter<'_>) -> FormatResult;
}

impl<T> AlbumLikeBoxed for T
where
    T: 'static + AlbumLike + Clone + Debug,
{
    fn clone_box(&self) -> Box<dyn AlbumLike> {
        Box::new(self.clone())
    }

    fn fmt_box(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt(f)
    }
}

impl Clone for Box<dyn AlbumLike> {
    fn clone(&self) -> Box<dyn AlbumLike> {
        self.clone_box()
    }
}

impl Debug for Box<dyn AlbumLike> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt_box(f)
    }
}
