extern crate ape;

use super::map::TagKey;
use super::picture::{Picture, PictureType};
use ape::{Item, ItemValue};
use core::convert::AsRef;
use eyre::{eyre, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Tag {
    tag: ape::Tag,
    // TODO: until upstream adds support:
    // https://github.com/rossnomann/rust-ape/issues/7
    separator: String,
}

impl crate::track::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::track::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: ape::read_from_path(path)?,
            // TODO:
            separator: ",".to_string(),
        }))
    }
}

fn value_to_strings(value: &ItemValue, separator: &String) -> Option<Vec<String>> {
    let val = match value {
        ItemValue::Text(str) => Some(str),
        _ => None,
    }?;
    Some(
        val.split(separator)
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
}

impl crate::track::Tag for Tag {
    fn clear(&mut self) {
        let map = self.get_all();
        for key in map.keys().into_iter() {
            self.tag.remove_item(key);
        }
    }
    fn separator(&self) -> Option<String> {
        Some(self.separator.clone())
    }
    fn get_str(&self, tag: &str) -> Option<Vec<String>> {
        let item = self.tag.item(tag)?;
        value_to_strings(&item.value, &self.separator)
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        self.tag
            .set_item(Item::from_text(key, values.join(&self.separator))?);
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        for item in self.tag.iter() {
            if let Some(vals) = value_to_strings(&item.value, &self.separator) {
                out.insert(item.key.clone(), vals);
            }
        }
        out
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        self.tag
            .iter()
            .filter_map(|item| match &item.value {
                ItemValue::Binary(b) => Some((item.key.clone(), b)),
                _ => None,
            })
            .map(|item| -> Result<Picture> {
                Ok(Picture {
                    mime_type: infer::get(&item.1.to_vec())
                        .ok_or(eyre!("Could not infer mime type from binary picture"))?
                        .to_string(),
                    picture_type: PictureType::CoverFront,
                    description: item.0,
                    data: item.1.to_vec(),
                })
            })
            .collect::<Result<Vec<_>>>()
    }

    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        unimplemented!()
    }

    fn str_to_key(&self, str: &str) -> Option<TagKey> {
        unimplemented!()
    }
    fn key_to_str(&self, key: TagKey) -> Option<&'static str> {
        match key {
            // TODO
            // TagKey::AcoustidID => Some(""),
            // TagKey::AcoustidIDFingerprint => Some(""),
            // TagKey::Album => Some(""),
            // TagKey::AlbumArtist => Some(""),
            // TagKey::AlbumArtistSortOrder => Some(""),
            // TagKey::AlbumSortOrder => Some(""),
            // TagKey::Arranger => Some(""),
            // TagKey::Artist => Some(""),
            // TagKey::Artists => Some(""),
            // TagKey::ArtistSortOrder => Some(""),
            // TagKey::ASIN => Some(""),
            // TagKey::Barcode => Some(""),
            // TagKey::BPM => Some(""),
            // TagKey::CatalogNumber => Some(""),
            // TagKey::Comment => Some(""),
            // TagKey::Compilation => Some(""),
            // TagKey::Composer => Some(""),
            // TagKey::ComposerSortOrder => Some(""),
            // TagKey::Conductor => Some(""),
            // TagKey::Copyright => Some(""),
            // TagKey::Director => Some(""),
            // TagKey::DiscNumber => Some(""),
            // TagKey::DiscSubtitle => Some(""),
            // TagKey::EncodedBy => Some(""),
            // TagKey::EncoderSettings => Some(""),
            // TagKey::Engineer => Some(""),
            // TagKey::GaplessPlayback => Some(""),
            // TagKey::Genre => Some(""),
            // TagKey::Grouping => Some(""),
            // TagKey::InitialKey => Some(""),
            // TagKey::ISRC => Some(""),
            // TagKey::Language => Some(""),
            // TagKey::License => Some(""),
            // TagKey::Liricist => Some(""),
            // TagKey::Lyrics => Some(""),
            // TagKey::Media => Some(""),
            // TagKey::MixDJ => Some(""),
            // TagKey::Mixer => Some(""),
            // TagKey::Mood => Some(""),
            // TagKey::Movement => Some(""),
            // TagKey::MovementCount => Some(""),
            // TagKey::MovementNumber => Some(""),
            // TagKey::MusicBrainzArtistID => Some(""),
            // TagKey::MusicBrainzDiscID => Some(""),
            // TagKey::MusicBrainzOriginalArtistID => Some(""),
            // TagKey::MusicBrainzOriginalReleaseID => Some(""),
            // TagKey::MusicBrainzRecordingID => Some(""),
            // TagKey::MusicBrainzReleaseArtistID => Some(""),
            // TagKey::MusicBrainzReleaseGroupID => Some(""),
            // TagKey::MusicBrainzReleaseID => Some(""),
            // TagKey::MusicBrainzTrackID => Some(""),
            // TagKey::MusicBrainzTRMID => Some(""),
            // TagKey::MusicBrainzWorkID => Some(""),
            // TagKey::MusicIPFingerprint => Some(""),
            // TagKey::MusicIPPUID => Some(""),
            // TagKey::OriginalAlbum => Some(""),
            // TagKey::OriginalArtist => Some(""),
            // TagKey::OriginalFilename => Some(""),
            // TagKey::OriginalReleaseDate => Some(""),
            // TagKey::OriginalReleaseYear => Some(""),
            // TagKey::Performer => Some(""),
            // TagKey::Podcast => Some(""),
            // TagKey::PodcastURL => Some(""),
            // TagKey::Producer => Some(""),
            // TagKey::Rating => Some(""),
            // TagKey::RecordLabel => Some(""),
            // TagKey::ReleaseCountry => Some(""),
            // TagKey::ReleaseDate => Some(""),
            // TagKey::ReleaseStatus => Some(""),
            // TagKey::ReleaseType => Some(""),
            // TagKey::Remixer => Some(""),
            // TagKey::ReplayGainAlbumGain => Some(""),
            // TagKey::ReplayGainAlbumPeak => Some(""),
            // TagKey::ReplayGainAlbumRange => Some(""),
            // TagKey::ReplayGainReferenceLoudness => Some(""),
            // TagKey::ReplayGainTrackGain => Some(""),
            // TagKey::ReplayGainTrackPeak => Some(""),
            // TagKey::ReplayGainTrackRange => Some(""),
            // TagKey::Script => Some(""),
            // TagKey::ShowName => Some(""),
            // TagKey::ShowNameSortOrder => Some(""),
            // TagKey::ShowWorkAndMovement => Some(""),
            // TagKey::Subtitle => Some(""),
            // TagKey::TotalDiscs => Some(""),
            // TagKey::TotalTracks => Some(""),
            // TagKey::TrackTitle => Some(""),
            // TagKey::TrackTitleSortOrder => Some(""),
            // TagKey::Website => Some(""),
            // TagKey::WorkTitle => Some(""),
            // TagKey::Writer => Some(""),
            _ => None,
        }
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        ape::write_to_path(&self.tag, path).map_err(|e| eyre!(e))
    }
}
