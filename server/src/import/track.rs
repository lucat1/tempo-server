use base::setting::Library;
use entity::TrackFormat;
use eyre::{bail, eyre, Result, WrapErr};
use std::{
    collections::HashMap,
    fs::copy,
    path::{Path, PathBuf},
};

#[cfg(feature = "ape")]
use tag::ape;
#[cfg(feature = "flac")]
use tag::flac;
#[cfg(feature = "id3")]
use tag::id3;
#[cfg(feature = "mp4")]
use tag::mp4;

use tag::TagKey;
use tag::{Picture, Tag, TagError, TagFrom};

#[derive(Clone, Debug)]
pub struct TrackFile {
    pub path: PathBuf,
    pub format: TrackFormat,
    pub tag: Box<dyn Tag>,
}

impl TrackFile {
    pub fn open(library: &Library, path: &PathBuf) -> Result<TrackFile> {
        let format = TrackFormat::from_path(path)
            .wrap_err(format!("Could not identify format for file: {path:?}"))?;
        let tag = match format {
            #[cfg(feature = "flac")]
            TrackFormat::Flac => flac::Tag::from_path(library, path),
            #[cfg(feature = "mp4")]
            TrackFormat::Mp4 => mp4::Tag::from_path(path),
            #[cfg(feature = "id3")]
            TrackFormat::Id3 => id3::Tag::from_path(library, path),
            #[cfg(feature = "ape")]
            TrackFormat::Ape => ape::Tag::from_path(path),
            _ => Err(eyre!("Unsupported format {}", String::from(format))),
        }
        .wrap_err(format!("Could not read metadata from file: {path:?}"))?;
        Ok(TrackFile {
            path: path.to_path_buf(),
            format,
            tag,
        })
    }

    pub fn get_tag(&self, key: TagKey) -> Vec<String> {
        self.tag.get_tag(key)
    }
    pub fn set_tag(&mut self, key: TagKey, values: Vec<String>) -> Result<(), TagError> {
        self.tag.set_tag(key, values)
    }

    pub fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        self.tag.set_pictures(pictures)
    }

    pub fn duplicate_to(&mut self, library: &Library, path: &Path) -> Result<()> {
        copy(&self.path, path)?;
        self.path = path.to_path_buf();
        self.tag = match self.format {
            #[cfg(feature = "flac")]
            TrackFormat::Flac => flac::Tag::from_path(library, &self.path),
            #[cfg(feature = "mp4")]
            TrackFormat::Mp4 => mp4::Tag::from_path(&self.path),
            #[cfg(feature = "id3")]
            TrackFormat::Id3 => id3::Tag::from_path(library, &self.path),
            #[cfg(feature = "ape")]
            TrackFormat::Ape => ape::Tag::from_path(&self.path),
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
