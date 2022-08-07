extern crate infer;
pub mod format;
pub mod flac;
pub mod m4a;
pub mod id3;

use std::fs::copy;
use std::path::{PathBuf, Path};
use std::collections::HashMap;
use core::convert::AsRef;
use std::fmt::{Debug, Formatter, Result as FormatResult};
use eyre::{Result, WrapErr};

use format::Format;

#[derive(Clone, Debug)]
pub struct Track {
    path: PathBuf,
    format: Format,
    tag: Box<dyn Tag>
}

impl Track {
    pub fn open(path: &PathBuf) -> Result<Box<Track>> {
        let format = Format::from_path(path)
            .wrap_err(format!("Could not identify format for file: {:?}", path))?;
        let tag = match format {
            Format::FLAC => flac::Tag::from_path(path),
            Format::M4A => m4a::Tag::from_path(path),
            Format::ID3 => id3::Tag::from_path(path),
        }.wrap_err(format!("Could not read metadata from file: {:?}", path))?;
        Ok(Box::new(Track{
            path: path.to_path_buf(),
            format,
            tag,
        }))
    }

    pub fn write(&mut self, path: &PathBuf) -> Result<()> {
        copy(&self.path, &path).wrap_err(format!("Error while copying file from {:?} to {:?}", self.path, path))?;
        self.tag.write_to_path(path).wrap_err(format!("Could not write tags to file: {:?}", path))
    }
}

pub trait TagFrom {
    fn from_path<P>(path: P) -> Result<Box<dyn Tag>>
        where P: AsRef<Path>;
}

pub trait TagClone {
    fn clone_box(&self) -> Box<dyn Tag>;
}

impl<T> TagClone for T where T: 'static + Tag + Clone {
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

pub trait Tag : TagClone {
    fn get_raw(&self, key: &str) -> Option<Vec<String>>;
    fn get_all_tags(&self) -> HashMap<String, Vec<String>>;

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()>;
}
