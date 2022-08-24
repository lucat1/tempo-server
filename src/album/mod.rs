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

pub trait ReleaseLike: ReleaseLikeBoxed {
    fn artist(&self) -> Result<Vec<String>>;
    fn title(&self) -> Result<Vec<String>>;
}

pub trait AlbumLike: ReleaseLike {
    fn tracks(&self) -> Vec<Box<dyn TrackLike>>;
}

impl ReleaseLike for RoughAlbum {
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
}

impl AlbumLike for RoughAlbum {
    fn tracks(&self) -> Vec<Box<dyn TrackLike>> {
        self.tracks
            .iter()
            .map(|t| Box::new(t.clone()) as Box<dyn TrackLike>)
            .collect::<Vec<_>>()
    }
}

pub trait ReleaseLikeBoxed {
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

impl Debug for Box<dyn AlbumLike> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt_box(f)
    }
}
