extern crate metaflac;

use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Tag {
    tag: metaflac::Tag,
}

impl crate::tag::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::tag::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: metaflac::Tag::read_from_path(path)?,
        }))
    }
}

impl crate::tag::Tag for Tag {
    fn get_str(&self, key: &str) -> Option<Vec<String>> {
        if let Some(values) = self.tag.get_vorbis(key) {
            let v: Vec<&str> = values.collect();
            if v.is_empty() {
                return None;
            }

            return Some(v.into_iter().map(|v| v.to_string()).collect());
        }
        None
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        self.tag.set_vorbis(key, values);
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        if let Some(vorbis) = self.tag.vorbis_comments() {
            // Get value of tag with proper separators
            vorbis.comments.iter().for_each(|(k, _)| {
                out.insert(k.to_string(), self.get_str(k).unwrap());
            });
        }
        out
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        Ok(self
            .tag
            .pictures()
            .map(|pic| Picture {
                mime_type: pic.mime_type.clone(),
                picture_type: match pic.picture_type {
                    metaflac::block::PictureType::Other => PictureType::Other,
                    metaflac::block::PictureType::Icon => PictureType::Icon,
                    metaflac::block::PictureType::OtherIcon => PictureType::OtherIcon,
                    metaflac::block::PictureType::CoverFront => PictureType::CoverFront,
                    metaflac::block::PictureType::CoverBack => PictureType::CoverBack,
                    metaflac::block::PictureType::Leaflet => PictureType::Leaflet,
                    metaflac::block::PictureType::Media => PictureType::Media,
                    metaflac::block::PictureType::LeadArtist => PictureType::LeadArtist,
                    metaflac::block::PictureType::Artist => PictureType::Artist,
                    metaflac::block::PictureType::Conductor => PictureType::Conductor,
                    metaflac::block::PictureType::Band => PictureType::Band,
                    metaflac::block::PictureType::Composer => PictureType::Composer,
                    metaflac::block::PictureType::Lyricist => PictureType::Lyricist,
                    metaflac::block::PictureType::RecordingLocation => {
                        PictureType::RecordingLocation
                    }
                    metaflac::block::PictureType::DuringRecording => PictureType::DuringRecording,
                    metaflac::block::PictureType::DuringPerformance => {
                        PictureType::DuringPerformance
                    }
                    metaflac::block::PictureType::ScreenCapture => PictureType::ScreenCapture,
                    metaflac::block::PictureType::BrightFish => PictureType::BrightFish,
                    metaflac::block::PictureType::Illustration => PictureType::Illustration,
                    metaflac::block::PictureType::BandLogo => PictureType::BandLogo,
                    metaflac::block::PictureType::PublisherLogo => PictureType::PublisherLogo,
                },
                description: pic.description.clone(),
                data: pic.data.clone(),
            })
            .collect::<Vec<_>>())
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag.write_to_path(path).map_err(|e| eyre!(e))
    }
}
