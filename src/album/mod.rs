use super::track::{TrackFile, TrackLike};
use crate::util::dedup;
use eyre::Result;
use std::fmt::{Debug, Formatter, Result as FormatResult};

pub trait ArtistLike: ArtistLikeBoxed {
    fn name(&self) -> String;
    fn mbid(&self) -> Option<String>;
    fn joinphrase(&self) -> Option<String>;
}

pub trait ArtistLikeBoxed {
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
    fn artists(&self) -> Vec<Box<dyn ArtistLike>>;
    fn title(&self) -> String;
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

impl Debug for Box<dyn ReleaseLike> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        self.fmt_box(f)
    }
}

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
