pub mod ape;
pub mod flac;
pub mod id3;
pub mod mp4;

pub mod file;
pub mod format;
pub mod map;
pub mod picture;

use core::convert::AsRef;
use eyre::{Report, Result};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FormatResult};
use std::path::{Path, PathBuf};

use self::map::TagKey;
use picture::Picture;

pub enum TagError {
    NotSupported,
    Other(Report),
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
