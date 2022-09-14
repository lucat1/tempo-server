extern crate infer;
pub mod ape;
pub mod flac;
pub mod id3;
pub mod mp4;

pub mod format;
pub mod map;
pub mod picture;

use super::models::{Artist, Artists, Track};
use super::util::{dedup, take_first};
use core::convert::AsRef;
use eyre::{eyre, Report, Result, WrapErr};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::fs::copy;
use std::path::{Path, PathBuf};
use std::time::Duration;

use format::Format;
use picture::Picture;

use self::map::TagKey;

#[derive(Clone, Debug)]
pub struct TrackFile {
    pub path: PathBuf,
    format: Format,
    tag: Box<dyn Tag>,
}

impl TrackFile {
    pub fn open(path: &PathBuf) -> Result<TrackFile> {
        let format = Format::from_path(path)
            .wrap_err(format!("Could not identify format for file: {:?}", path))?;
        let tag = match format {
            Format::FLAC => flac::Tag::from_path(path),
            Format::MP4 => mp4::Tag::from_path(path),
            Format::ID3 => id3::Tag::from_path(path),
            Format::APE => ape::Tag::from_path(path),
        }
        .wrap_err(format!("Could not read metadata from file: {:?}", path))?;
        Ok(TrackFile {
            path: path.to_path_buf(),
            format,
            tag,
        })
    }

    pub fn get_tag(&self, key: TagKey) -> Result<Vec<String>> {
        let keystr = self.tag.key_to_str(key).ok_or(eyre!(
            "The {:?} key is not supported in the output format {:?}",
            key,
            self.format
        ))?;
        self.tag
            .get_str(keystr)
            .ok_or(eyre!("Could not read tag {:?} as {}", key, keystr))
    }

    pub fn set_tag(&mut self, key: TagKey, values: Vec<String>) -> Result<()> {
        let keystr = self.tag.key_to_str(key).ok_or(eyre!(
            "The {:?} key is not supported in the output format {:?}",
            key,
            self.format
        ))?;
        self.tag.set_str(keystr, values)
    }

    pub fn tags(&self) -> HashMap<TagKey, Vec<String>> {
        let mut map = HashMap::new();
        for (key, value) in self.tag.get_all() {
            if let Some(k) = self.tag.str_to_key(key.as_str()) {
                map.insert(k, value);
            }
        }
        map
    }

    pub fn duplicate_to(&mut self, path: &PathBuf) -> Result<()> {
        copy(&self.path, path)?;
        self.path = path.to_path_buf();
        self.tag = match self.format {
            Format::FLAC => flac::Tag::from_path(&self.path),
            Format::MP4 => mp4::Tag::from_path(&self.path),
            Format::ID3 => id3::Tag::from_path(&self.path),
            Format::APE => ape::Tag::from_path(&self.path),
        }?;
        Ok(())
    }

    pub fn write(&mut self) -> Result<()> {
        self.tag
            .write_to_path(&self.path)
            .wrap_err(format!("Could not write tags to file: {:?}", self.path))
    }

    pub fn apply(&mut self, track: Track) -> Result<()> {
        // work
        if let Some(id) = track.mbid {
            self.set_tag(TagKey::MusicBrainzTrackID, vec![id])?;
        }
        if let Some(release) = track.release {
            if let Some(rel_id) = &release.mbid {
                self.set_tag(TagKey::MusicBrainzReleaseID, vec![rel_id.clone()])?;
            }
            if let Some(rel_asin) = &release.asin {
                self.set_tag(TagKey::ASIN, vec![rel_asin.to_string()])?;
            }
            self.set_tag(TagKey::Album, vec![release.title.clone()])?;
            self.set_tag(TagKey::AlbumArtist, release.artists.names())?;
            self.set_tag(TagKey::MusicBrainzReleaseArtistID, release.artists.ids())?;
            if let Some(discs) = release.discs {
                self.set_tag(TagKey::TotalDiscs, vec![discs.to_string()])?;
            }
        }
        self.set_tag(TagKey::TrackTitle, vec![track.title])?;

        // artists
        self.set_tag(TagKey::Artists, track.artists.names())?;
        self.set_tag(TagKey::MusicBrainzArtistID, track.artists.ids())?;
        self.set_tag(TagKey::ArtistSortOrder, vec![track.artists.sort_order()])?;
        if let Some(len) = track.length {
            self.set_tag(TagKey::Duration, vec![len.as_secs().to_string()])?;
        }
        if let Some(disc) = track.disc {
            self.set_tag(TagKey::DiscNumber, vec![disc.to_string()])?;
        }
        if let Some(disc_mbid) = track.disc_mbid {
            self.set_tag(TagKey::MusicBrainzDiscID, vec![disc_mbid.to_string()])?;
        }
        if let Some(number) = track.number {
            self.set_tag(TagKey::TrackNumber, vec![number.to_string()])?;
        }
        self.set_tag(TagKey::Genre, track.genres)?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.tag.clear()?;
        Ok(())
    }

    pub fn ext(&self) -> &'static str {
        self.format.ext()
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

pub trait TagFrom {
    fn from_path<P>(path: P) -> Result<Box<dyn Tag>>
    where
        P: AsRef<Path>;
}

pub trait TagClone: Send {
    fn clone_box(&self) -> Box<dyn Tag>;
}

impl<T> TagClone for T
where
    T: 'static + Tag + Clone,
{
    fn clone_box(&self) -> Box<dyn Tag> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Tag> {
    fn clone(&self) -> Box<dyn Tag> {
        self.clone_box()
    }
}

impl Debug for Box<dyn Tag> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        let mut str = f.debug_struct("Tag");
        for (k, v) in self.get_all() {
            str.field(&k, &v);
        }
        str.field("pictures", &self.get_pictures());
        str.finish()
    }
}

pub trait Tag: TagClone {
    fn separator(&self) -> Option<String>;

    fn clear(&mut self) -> Result<()>;
    fn get_str(&self, key: &str) -> Option<Vec<String>>;
    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()>;
    fn get_all(&self) -> HashMap<String, Vec<String>>;
    fn get_pictures(&self) -> Result<Vec<Picture>>;
    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()>;

    fn str_to_key(&self, str: &str) -> Option<TagKey>;
    fn key_to_str(&self, key: TagKey) -> Option<&'static str>;

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()>;
}

impl TrackFile {
    pub fn artists(&self) -> Result<Vec<String>> {
        Ok(dedup(self.get_tag(TagKey::AlbumArtist)?))
    }
    pub fn album_artists(&self) -> Result<Vec<String>> {
        self.get_tag(TagKey::AlbumArtist)
    }
    pub fn album_title(&self) -> Result<String> {
        take_first(
            self.get_tag(TagKey::Album)?,
            "Track has no album title".to_string(),
        )
    }
}

impl TryFrom<TrackFile> for Track {
    type Error = Report;
    fn try_from(file: TrackFile) -> Result<Self> {
        let titles = file
            .get_tag(TagKey::TrackTitle)
            .wrap_err(eyre!("Could not read title tag"))?;
        let title = take_first(titles, format!("Track {:?} has no title", file.path))?;
        let mbid = file
            .get_tag(TagKey::MusicBrainzTrackID)
            .ok()
            .map_or(None, |ids| ids.first().map(|f| f.to_string()));
        let length = file
            .get_tag(TagKey::Duration)
            .map_or(None, |d| d.first().map(|d| d.to_string()))
            .map_or(None, |d| d.parse::<u64>().ok())
            .map(|d| Duration::from_secs(d));
        let artists = file.artists().ok().map_or(vec![], |a| {
            a.iter()
                .map(|name| Artist {
                    mbid: None,
                    name: name.to_string(),
                    join_phrase: file.tag.separator(),
                    // TODO: take a look into artist sort order
                    sort_name: None,
                })
                .collect::<Vec<_>>()
        });
        let disc = file
            .get_tag(TagKey::DiscNumber)
            .ok()
            .map_or(None, |t| t.first().map(|d| d.to_string()))
            .map_or(None, |d| d.parse::<u64>().ok());
        let disc_mbid = file
            .get_tag(TagKey::MusicBrainzDiscID)
            .ok()
            .map_or(None, |ids| ids.first().map(|f| f.to_string()));
        let number = file
            .get_tag(TagKey::TrackNumber)
            .ok()
            .map_or(None, |t| t.first().map(|n| n.to_string()))
            .map_or(None, |d| d.parse::<u64>().ok());
        Ok(Track {
            mbid,
            title,
            artists,
            length,
            disc,
            disc_mbid,
            number,
            // TODO: fetch from tags, decide on how (and if) to split
            genres: vec![],
            release: None,
        })
    }
}
