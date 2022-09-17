pub mod ape;
pub mod flac;
pub mod id3;
pub mod mp4;

pub mod format;
pub mod map;
pub mod picture;

use super::models::{Artist, Artists, Track};
use super::util::dedup;
use chrono::Datelike;
use core::convert::AsRef;
use eyre::{eyre, Report, Result, WrapErr};
use itertools::Itertools;
use log::debug;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::fs::copy;
use std::path::{Path, PathBuf};
use std::time::Duration;

use self::map::TagKey;
use format::Format;
use picture::Picture;

#[derive(Clone, Debug)]
pub struct TrackFile {
    pub path: PathBuf,
    format: Format,
    tag: Box<dyn Tag>,
}

pub enum TagError {
    NotSupported,
    Other(Report),
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

    pub fn get_tag(&self, key: TagKey) -> Vec<String> {
        let keystrs = self.tag.key_to_str(key);
        if keystrs.is_empty() {
            debug!(
                "The {:?} key is not supported in the output format {:?}",
                key, self.format
            );
            return vec![];
        }
        keystrs
            .into_iter()
            .filter_map(|keystr| self.tag.get_str(keystr))
            .flatten()
            .collect()
    }

    pub fn set_tag(&mut self, key: TagKey, values: Vec<String>) -> Result<(), TagError> {
        let keystrs = self.tag.key_to_str(key);
        if keystrs.is_empty() {
            return Err(TagError::NotSupported);
        }
        keystrs
            .into_iter()
            .map(|keystr| {
                self.tag
                    .set_str(keystr, values.clone())
                    .map_err(|e| TagError::Other(e))
            })
            .collect()
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

    fn ignore_unsupported(r: Result<(), TagError>) -> Result<()> {
        match r {
            Err(TagError::NotSupported) => Ok(()),
            Err(TagError::Other(v)) => Err(eyre!(v)),
            Ok(v) => Ok(v),
        }
    }

    pub fn apply(&mut self, track: Track) -> Result<()> {
        if let Some(id) = track.mbid {
            Self::ignore_unsupported(self.set_tag(TagKey::MusicBrainzTrackID, vec![id]))?;
        }
        if let Some(release) = track.release {
            if let Some(rel_id) = &release.mbid {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::MusicBrainzReleaseID, vec![rel_id.clone()]),
                )?;
            }
            if let Some(rel_group_id) = &release.release_group_mbid {
                Self::ignore_unsupported(self.set_tag(
                    TagKey::MusicBrainzReleaseGroupID,
                    vec![rel_group_id.clone()],
                ))?;
            }
            if let Some(rel_asin) = &release.asin {
                Self::ignore_unsupported(self.set_tag(TagKey::ASIN, vec![rel_asin.to_string()]))?;
            }
            if let Some(rel_country) = &release.country {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::ReleaseCountry, vec![rel_country.to_string()]),
                )?;
            }
            if let Some(rel_label) = &release.label {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::RecordLabel, vec![rel_label.to_string()]),
                )?;
            }
            if let Some(rel_catno) = &release.catalog_no {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::CatalogNumber, vec![rel_catno.to_string()]),
                )?;
            }
            if let Some(rel_status) = &release.status {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::ReleaseStatus, vec![rel_status.to_string()]),
                )?;
            }
            if let Some(rel_type) = &release.release_type {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::ReleaseType, vec![rel_type.to_string()]),
                )?;
            }
            if let Some(rel_date) = &release.date {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::ReleaseDate, vec![rel_date.to_string()]),
                )?;
                Self::ignore_unsupported(
                    self.set_tag(TagKey::ReleaseYear, vec![rel_date.year().to_string()]),
                )?;
            }
            if let Some(rel_original_date) = &release.original_date {
                Self::ignore_unsupported(self.set_tag(
                    TagKey::OriginalReleaseDate,
                    vec![rel_original_date.to_string()],
                ))?;
                Self::ignore_unsupported(self.set_tag(
                    TagKey::OriginalReleaseYear,
                    vec![rel_original_date.year().to_string()],
                ))?;
            }
            if let Some(rel_script) = &release.script {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::Script, vec![rel_script.to_string()]),
                )?;
            }
            if let Some(rel_media) = &release.media {
                Self::ignore_unsupported(self.set_tag(TagKey::Media, vec![rel_media.to_string()]))?;
            }
            Self::ignore_unsupported(self.set_tag(TagKey::Album, vec![release.title.clone()]))?;
            Self::ignore_unsupported(self.set_tag(TagKey::AlbumArtist, release.artists.names()))?;
            Self::ignore_unsupported(
                self.set_tag(TagKey::AlbumArtistSortOrder, release.artists.sort_order()),
            )?;
            Self::ignore_unsupported(
                self.set_tag(TagKey::MusicBrainzReleaseArtistID, release.artists.ids()),
            )?;
            if let Some(discs) = release.discs {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::TotalDiscs, vec![discs.to_string()]),
                )?;
            }
            if let Some(tracks) = release.tracks {
                Self::ignore_unsupported(
                    self.set_tag(TagKey::TotalTracks, vec![tracks.to_string()]),
                )?;
            }
        }
        Self::ignore_unsupported(self.set_tag(TagKey::TrackTitle, vec![track.title]))?;

        // artists
        Self::ignore_unsupported(self.set_tag(TagKey::Artists, track.artists.names()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::MusicBrainzArtistID, track.artists.ids()))?;
        Self::ignore_unsupported(
            self.set_tag(TagKey::ArtistSortOrder, track.artists.sort_order()),
        )?;
        if let Some(len) = track.length {
            Self::ignore_unsupported(
                self.set_tag(TagKey::Duration, vec![len.as_secs().to_string()]),
            )?;
        }
        if let Some(disc) = track.disc {
            Self::ignore_unsupported(self.set_tag(TagKey::DiscNumber, vec![disc.to_string()]))?;
        }
        if let Some(disc_mbid) = track.disc_mbid {
            Self::ignore_unsupported(
                self.set_tag(TagKey::MusicBrainzDiscID, vec![disc_mbid.to_string()]),
            )?;
        }
        if let Some(number) = track.number {
            Self::ignore_unsupported(self.set_tag(TagKey::TrackNumber, vec![number.to_string()]))?;
        }
        Self::ignore_unsupported(self.set_tag(TagKey::Genre, track.genres))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Performer, track.performers.instruments()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Engineer, track.engigneers.names()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Mixer, track.mixers.names()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Producer, track.producers.names()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Lyricist, track.lyricists.names()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Writer, track.writers.names()))?;
        Self::ignore_unsupported(self.set_tag(TagKey::Composer, track.composers.names()))?;
        Self::ignore_unsupported(
            self.set_tag(TagKey::ComposerSortOrder, track.composers.sort_order()),
        )?;
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

    pub fn artists(&self) -> Vec<String> {
        dedup(
            self.tracks
                .iter()
                .map(|t| t.album_artists())
                .flatten()
                .map(|s| s.clone())
                .collect(),
        )
    }

    pub fn titles(&self) -> Vec<String> {
        dedup(self.tracks.iter().map(|t| t.album_title()).collect())
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
    fn key_to_str(&self, key: TagKey) -> Vec<&'static str>;

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()>;
}

impl TrackFile {
    pub fn artists(&self) -> Vec<String> {
        self.get_tag(TagKey::Artists)
    }
    pub fn album_artists(&self) -> Vec<String> {
        self.get_tag(TagKey::AlbumArtist)
    }
    pub fn album_title(&self) -> String {
        self.get_tag(TagKey::Album)
            .first()
            .map_or("".to_string(), |t| t.to_string())
    }
}

fn artists_with_name(name: String, sep: Option<String>) -> Vec<Artist> {
    match sep {
        Some(ref s) => name.split(s.as_str()).map(|s| s.to_string()).collect_vec(),
        None => vec![name],
    }
    .into_iter()
    .map(|s| Artist {
        mbid: None,
        name: s.to_string(),
        join_phrase: sep.clone(),
        // TODO: take a look into artist sort order
        sort_name: None,
        instruments: vec![],
    })
    .collect()
}

fn artists_from_tag(file: &TrackFile, key: TagKey) -> Vec<Artist> {
    file.get_tag(key)
        .iter()
        .map(|name| artists_with_name(name.to_string(), file.tag.separator()))
        .flatten()
        .collect()
}

impl TryFrom<TrackFile> for Track {
    type Error = Report;
    fn try_from(file: TrackFile) -> Result<Self> {
        let title = file
            .get_tag(TagKey::TrackTitle)
            .first()
            .ok_or(eyre!("Could not read title tag"))?
            .to_string();
        let mbid = file
            .get_tag(TagKey::MusicBrainzTrackID)
            .first()
            .map(|id| id.to_string());
        let length = file
            .get_tag(TagKey::Duration)
            .first()
            .map_or(None, |d| d.parse::<u64>().ok())
            .map(|d| Duration::from_secs(d));
        let artists = artists_from_tag(&file, TagKey::Artists);
        let disc = file
            .get_tag(TagKey::DiscNumber)
            .first()
            .map_or(None, |d| d.parse::<u64>().ok());
        let disc_mbid = file
            .get_tag(TagKey::MusicBrainzDiscID)
            .first()
            .map(|f| f.to_string());
        let number = file
            .get_tag(TagKey::TrackNumber)
            .first()
            .map_or(None, |d| d.parse::<u64>().ok());
        let performers = artists_from_tag(&file, TagKey::Performer);
        let engigneers = artists_from_tag(&file, TagKey::Engineer);
        let mixers = artists_from_tag(&file, TagKey::Mixer);
        let producers = artists_from_tag(&file, TagKey::Producer);
        let lyricists = artists_from_tag(&file, TagKey::Lyrics);
        let writers = artists_from_tag(&file, TagKey::Writer);
        let composers = artists_from_tag(&file, TagKey::Composer);
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
            performers,
            engigneers,
            mixers,
            producers,
            lyricists,
            writers,
            composers,
        })
    }
}
