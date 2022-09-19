use chrono::Datelike;
use eyre::{eyre, Report, Result, WrapErr};
use itertools::Itertools;
use log::{debug, warn};
use std::fs::copy;
use std::path::PathBuf;
use std::time::Duration;

use super::ape;
use super::flac;
use super::format::Format;
use super::id3;
use super::mp4;
use super::Picture;
use super::TagFrom;
use super::{Tag, TagError};
use crate::models::UNKNOWN_TITLE;
use crate::models::{Artist, Artists, GroupTracks, Release, Track};
use crate::track::TagKey;
use crate::util::{dedup, maybe_date};
use crate::SETTINGS;

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

    pub fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        self.tag.set_pictures(pictures)
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
        let settings = SETTINGS.get().ok_or(eyre!("Could not get settings"))?;
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
            Self::ignore_unsupported(
                self.set_tag(TagKey::AlbumSortOrder, vec![release.title.clone()]),
            )?;
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
        Self::ignore_unsupported(self.set_tag(TagKey::Artist, track.artists.names()))?;
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
        Self::ignore_unsupported(self.set_tag(
            TagKey::Genre,
            match settings.tagging.genre_limit {
                None => track.genres,
                Some(l) => track.genres.into_iter().take(l).collect(),
            },
        ))?;
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

fn artists_from_tag(tracks: &Vec<TrackFile>, tag: TagKey) -> Vec<Artist> {
    let separator = match tracks.first() {
        Some(t) => t.tag.separator(),
        None => return vec![],
    };
    dedup(tracks.iter().map(|t| t.get_tag(tag)).flatten().collect())
        .iter()
        .map(|name| artists_with_name(name.to_string(), separator.clone()))
        .flatten()
        .collect()
}

fn first_tag(tracks: &Vec<TrackFile>, tag: TagKey) -> Option<String> {
    let options = dedup(tracks.iter().map(|t| t.get_tag(tag)).flatten().collect());
    if options.len() > 1 {
        warn!(
            "Multiple ({}) unique tag values for {:?} in the given tracks",
            options.len(),
            tag
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
                .map_or(None, |d| d.parse::<u64>().ok())
                .map(|d| Duration::from_secs(d)),
            disc: first_tag(&file_singleton, TagKey::DiscNumber)
                .map_or(None, |d| d.parse::<u64>().ok()),
            disc_mbid: first_tag(&file_singleton, TagKey::MusicBrainzDiscID),
            number: first_tag(&file_singleton, TagKey::TrackNumber)
                .map_or(None, |d| d.parse::<u64>().ok()),
            // TODO: fetch from tags, the eventual splitting should be handled track::Tag side
            genres: vec![],
            release: None,
            performers: artists_from_tag(&file_singleton, TagKey::Performer),
            engigneers: artists_from_tag(&file_singleton, TagKey::Engineer),
            mixers: artists_from_tag(&file_singleton, TagKey::Mixer),
            producers: artists_from_tag(&file_singleton, TagKey::Producer),
            lyricists: artists_from_tag(&file_singleton, TagKey::Lyrics),
            writers: artists_from_tag(&file_singleton, TagKey::Writer),
            composers: artists_from_tag(&file_singleton, TagKey::Composer),
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
            title: first_tag(&tracks, TagKey::Album).unwrap_or(UNKNOWN_TITLE.to_string()),
            artists: artists_from_tag(&tracks, TagKey::AlbumArtist),
            discs: first_tag(&tracks, TagKey::TotalDiscs).map_or(None, |d| d.parse::<u64>().ok()),
            media: first_tag(&tracks, TagKey::Media),
            tracks: first_tag(&tracks, TagKey::TotalTracks).map_or(None, |d| d.parse::<u64>().ok()),
            country: first_tag(&tracks, TagKey::ReleaseCountry),
            label: first_tag(&tracks, TagKey::RecordLabel),
            catalog_no: first_tag(&tracks, TagKey::CatalogNumber),
            status: first_tag(&tracks, TagKey::ReleaseStatus),
            release_type: first_tag(&tracks, TagKey::ReleaseType),
            date: maybe_date(
                first_tag(&tracks, TagKey::ReleaseDate).or(first_tag(&tracks, TagKey::ReleaseYear)),
            ),
            original_date: maybe_date(
                first_tag(&tracks, TagKey::OriginalReleaseDate)
                    .or(first_tag(&tracks, TagKey::OriginalReleaseYear)),
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
