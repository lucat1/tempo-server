extern crate id3;

use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use id3::frame::ExtendedText;
use id3::frame::PictureType as ID3PictureType;
use id3::{Content, Frame, TagLike, Version};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Tag {
    tag: id3::Tag,
    separator: String,
}

impl crate::tag::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::tag::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: id3::Tag::read_from_path(path)?,
            // TODO
            separator: ",".to_string(),
        }))
    }
}

impl crate::tag::Tag for Tag {
    fn get_str(&self, key: &str) -> Option<Vec<String>> {
        if key.len() != 4 {
            if let Some(t) = self.tag.extended_texts().find(|t| t.description == key) {
                return Some(
                    t.value
                        .split(&self.separator)
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>(),
                );
            }
            return None;
        } else if let Some(t) = self.tag.get(key) {
            if let Some(content) = t.content().text() {
                Some(content.split(&self.separator).map(String::from).collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        let val = values.join(&self.separator);
        let frame = if key.len() != 4 {
            ExtendedText {
                description: key.to_string(),
                value: val,
            }
            .into()
        } else {
            Frame::text(key, val)
        };
        self.tag.add_frame(frame);
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut tags = HashMap::new();
        for frame in self.tag.frames() {
            if let Content::Text(v) = frame.content() {
                tags.insert(
                    frame.id().to_owned(),
                    v.split(&self.separator).map(String::from).collect(),
                );
            }
        }
        // Add TXXX
        for extended in self.tag.extended_texts() {
            tags.insert(
                extended.description.to_string(),
                extended
                    .value
                    .split(&self.separator)
                    .map(String::from)
                    .collect(),
            );
        }
        tags
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        Ok(self
            .tag
            .pictures()
            .map(|pic| Picture {
                mime_type: pic.mime_type.clone(),
                picture_type: match pic.picture_type {
                    ID3PictureType::Other => PictureType::Other,
                    ID3PictureType::Icon => PictureType::Icon,
                    ID3PictureType::OtherIcon => PictureType::OtherIcon,
                    ID3PictureType::CoverFront => PictureType::CoverFront,
                    ID3PictureType::CoverBack => PictureType::CoverBack,
                    ID3PictureType::Leaflet => PictureType::Leaflet,
                    ID3PictureType::Media => PictureType::Media,
                    ID3PictureType::LeadArtist => PictureType::LeadArtist,
                    ID3PictureType::Artist => PictureType::Artist,
                    ID3PictureType::Conductor => PictureType::Conductor,
                    ID3PictureType::Band => PictureType::Band,
                    ID3PictureType::Composer => PictureType::Composer,
                    ID3PictureType::Lyricist => PictureType::Lyricist,
                    ID3PictureType::RecordingLocation => PictureType::RecordingLocation,
                    ID3PictureType::DuringRecording => PictureType::DuringRecording,
                    ID3PictureType::DuringPerformance => PictureType::DuringPerformance,
                    ID3PictureType::ScreenCapture => PictureType::ScreenCapture,
                    ID3PictureType::BrightFish => PictureType::BrightFish,
                    ID3PictureType::Illustration => PictureType::Illustration,
                    ID3PictureType::BandLogo => PictureType::BandLogo,
                    ID3PictureType::PublisherLogo => PictureType::PublisherLogo,
                    ID3PictureType::Undefined(_) => PictureType::Other,
                },
                description: pic.description.clone(),
                data: pic.data.clone(),
            })
            .collect::<Vec<_>>())
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag
            .write_to_path(path, Version::Id3v24)
            .map_err(|e| eyre!(e))
    }
}
