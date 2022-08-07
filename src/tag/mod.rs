extern crate infer;
pub mod ape;
pub mod flac;
pub mod format;
pub mod id3;
pub mod mp4;

use core::convert::AsRef;
use eyre::{Result, WrapErr};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::fs::copy;
use std::path::{Path, PathBuf};

use format::Format;

#[derive(Clone, Debug)]
pub struct Track {
    path: PathBuf,
    format: Format,
    tag: Box<dyn Tag>,
}

impl Track {
    pub fn open(path: &PathBuf) -> Result<Box<Track>> {
        let format = Format::from_path(path)
            .wrap_err(format!("Could not identify format for file: {:?}", path))?;
        let tag = match format {
            Format::FLAC => flac::Tag::from_path(path),
            Format::MP4 => mp4::Tag::from_path(path),
            Format::ID3 => id3::Tag::from_path(path),
            Format::APE => ape::Tag::from_path(path),
        }
        .wrap_err(format!("Could not read metadata from file: {:?}", path))?;
        Ok(Box::new(Track {
            path: path.to_path_buf(),
            format,
            tag,
        }))
    }

    fn r#move(&mut self, path: &PathBuf) -> Result<PathBuf> {
        copy(&self.path, path)?;
        let path = self.path.clone();
        self.path = path.to_path_buf();
        Ok(path)
    }

    fn write(&mut self) -> Result<()> {
        self.tag
            .write_to_path(&self.path)
            .wrap_err(format!("Could not write tags to file: {:?}", self.path))
    }
}

pub trait TagFrom {
    fn from_path<P>(path: P) -> Result<Box<dyn Tag>>
    where
        P: AsRef<Path>;
}

pub trait TagClone {
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
        for (k, v) in self.get_all_tags() {
            str.field(&k, &v);
        }
        str.finish()
    }
}

pub trait Tag: TagClone {
    fn get_raw(&self, key: &str) -> Option<Vec<String>>;
    fn get_all_tags(&self) -> HashMap<String, Vec<String>>;

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()>;
}
