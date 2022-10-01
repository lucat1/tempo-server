use eyre::{bail, eyre, Report, Result, WrapErr};
use itertools::Itertools;
use log::{debug, warn};
use std::collections::HashMap;
use std::fs::copy;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(feature = "ape")]
use super::ape;
#[cfg(feature = "flac")]
use super::flac;
use super::format::Format;
#[cfg(feature = "id3")]
use super::id3;
#[cfg(feature = "mp4")]
use super::mp4;
use super::Picture;
use super::TagFrom;
use super::{Tag, TagError};
use crate::models::UNKNOWN_TITLE;
use crate::models::{Artist, GroupTracks, Release, Track};
use crate::track::TagKey;
use crate::util::{dedup, maybe_date};

#[derive(Clone, Debug)]
pub struct TrackFile {
    pub path: PathBuf,
    pub format: Format,
    tag: Box<dyn Tag>,
}

impl TrackFile {
    pub fn open(path: &PathBuf) -> Result<TrackFile> {
        let format = Format::from_path(path)
            .wrap_err(format!("Could not identify format for file: {:?}", path))?;
        let tag = match format {
            #[cfg(feature = "flac")]
            Format::Flac => flac::Tag::from_path(path),
            #[cfg(feature = "mp4")]
            Format::Mp4 => mp4::Tag::from_path(path),
            #[cfg(feature = "id3")]
            Format::Id3 => id3::Tag::from_path(path),
            #[cfg(feature = "ape")]
            Format::Ape => ape::Tag::from_path(path),
            _ => bail!("Unsupported format {}", String::from(format)),
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
        keystrs.into_iter().try_for_each(|keystr| {
            self.tag
                .set_str(keystr, values.clone())
                .map_err(TagError::Other)
        })
    }

    pub fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        self.tag.set_pictures(pictures)
    }

    pub fn duplicate_to(&mut self, path: &PathBuf) -> Result<()> {
        copy(&self.path, path)?;
        self.path = path.to_path_buf();
        self.tag = match self.format {
            #[cfg(feature = "flac")]
            Format::Flac => flac::Tag::from_path(&self.path),
            #[cfg(feature = "mp4")]
            Format::Mp4 => mp4::Tag::from_path(&self.path),
            #[cfg(feature = "id3")]
            Format::Id3 => id3::Tag::from_path(&self.path),
            #[cfg(feature = "ape")]
            Format::Ape => ape::Tag::from_path(&self.path),
            _ => bail!("Unsupported format {}", String::from(self.format)),
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

    pub fn apply(&mut self, tags: HashMap<TagKey, Vec<String>>) -> Result<()> {
        for (k, v) in tags.into_iter() {
            Self::ignore_unsupported(self.set_tag(k, v))?;
        }
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.tag.clear()?;
        Ok(())
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
        name: s,
        join_phrase: sep.clone(),
        // TODO: take a look into artist sort order
        sort_name: None,
        instruments: vec![],
    })
    .collect()
}

fn artists_from_tag(tracks: &[TrackFile], tag: TagKey) -> Vec<Artist> {
    let separator = match tracks.first() {
        Some(t) => t.tag.separator(),
        None => return vec![],
    };
    dedup(tracks.iter().flat_map(|t| t.get_tag(tag)).collect())
        .iter()
        .flat_map(|name| artists_with_name(name.to_string(), separator.clone()))
        .collect()
}

fn first_tag(tracks: &[TrackFile], tag: TagKey) -> Option<String> {
    let options = dedup(tracks.iter().flat_map(|t| t.get_tag(tag)).collect());
    if options.len() > 1 {
        warn!(
            "Multiple ({}) unique tag values for {:?} in the given tracks ({})",
            options.len(),
            tag,
            // TODO
            options.join(", ")
        );
    }
    options.first().map(|f| f.to_string())
}

impl TryFrom<TrackFile> for Track {
    type Error = Report;
    fn try_from(file: TrackFile) -> Result<Self> {
        let file_singleton = vec![file];
        Ok(Track {
            mbid: first_tag(&file_singleton, TagKey::MusicBrainzTrackID),
            title: first_tag(&file_singleton, TagKey::TrackTitle)
                .ok_or(eyre!("A track doesn't have any title"))?,
            artists: artists_from_tag(&file_singleton, TagKey::Artists),
            length: first_tag(&file_singleton, TagKey::Duration)
                .and_then(|d| d.parse::<u64>().ok())
                .map(Duration::from_secs),
            disc: first_tag(&file_singleton, TagKey::DiscNumber)
                .and_then(|d| d.parse::<u64>().ok()),
            disc_mbid: first_tag(&file_singleton, TagKey::MusicBrainzDiscID),
            number: first_tag(&file_singleton, TagKey::TrackNumber)
                .and_then(|d| d.parse::<u64>().ok()),
            // TODO: fetch from tags, the eventual splitting should be handled track::Tag side
            genres: file_singleton[0].get_tag(TagKey::Genre),
            release: None,
            performers: artists_from_tag(&file_singleton, TagKey::Performer),
            engigneers: artists_from_tag(&file_singleton, TagKey::Engineer),
            mixers: artists_from_tag(&file_singleton, TagKey::Mixer),
            producers: artists_from_tag(&file_singleton, TagKey::Producer),
            lyricists: artists_from_tag(&file_singleton, TagKey::Lyrics),
            writers: artists_from_tag(&file_singleton, TagKey::Writer),
            composers: artists_from_tag(&file_singleton, TagKey::Composer),

            format: Some(file_singleton[0].format),
            path: Some(file_singleton[0].path.clone()),
        })
    }
}

impl TryFrom<Vec<TrackFile>> for Release {
    type Error = Report;
    fn try_from(tracks: Vec<TrackFile>) -> Result<Self, Self::Error> {
        Ok(Release {
            mbid: first_tag(&tracks, TagKey::MusicBrainzReleaseID),
            release_group_mbid: first_tag(&tracks, TagKey::MusicBrainzReleaseGroupID),
            asin: first_tag(&tracks, TagKey::ASIN),
            title: first_tag(&tracks, TagKey::Album).unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists: artists_from_tag(&tracks, TagKey::AlbumArtist),
            discs: first_tag(&tracks, TagKey::TotalDiscs).and_then(|d| d.parse::<u64>().ok()),
            media: first_tag(&tracks, TagKey::Media),
            tracks: first_tag(&tracks, TagKey::TotalTracks).and_then(|d| d.parse::<u64>().ok()),
            country: first_tag(&tracks, TagKey::ReleaseCountry),
            label: first_tag(&tracks, TagKey::RecordLabel),
            catalog_no: first_tag(&tracks, TagKey::CatalogNumber),
            status: first_tag(&tracks, TagKey::ReleaseStatus),
            release_type: first_tag(&tracks, TagKey::ReleaseType),
            date: maybe_date(
                first_tag(&tracks, TagKey::ReleaseDate)
                    .or_else(|| first_tag(&tracks, TagKey::ReleaseYear)),
            ),
            original_date: maybe_date(
                first_tag(&tracks, TagKey::OriginalReleaseDate)
                    .or_else(|| first_tag(&tracks, TagKey::OriginalReleaseYear)),
            ),
            script: first_tag(&tracks, TagKey::Script),
        })
    }
}

impl GroupTracks for Vec<TrackFile> {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)> {
        Ok((
            self.clone().try_into()?,
            self.into_iter()
                .map(|t| t.try_into())
                .collect::<Result<Vec<_>>>()?,
        ))
    }
}
