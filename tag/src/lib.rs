#[cfg(feature = "ape")]
pub mod ape;
#[cfg(feature = "flac")]
pub mod flac;
#[cfg(feature = "id3")]
pub mod id3;
#[cfg(feature = "mp4")]
pub mod mp4;

pub mod file;
pub mod format;
pub mod key;
pub mod map;
pub mod picture;

pub use core::convert::AsRef;
pub use eyre::{Report, Result};
pub use log::debug;
pub use std::collections::HashMap;
pub use std::fmt::{Debug, Formatter, Result as FormatResult};
pub use std::path::Path;

pub use file::TrackFile;
pub use format::Format;
pub use key::TagKey;
pub use picture::Picture;

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
    fn format(&self) -> Format;
    fn separator(&self) -> Option<String>;

    fn clear(&mut self) -> Result<()>;
    fn get_all(&self) -> HashMap<String, Vec<String>>;
    fn get_pictures(&self) -> Result<Vec<Picture>>;
    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()>;

    fn get_str(&self, key: &str) -> Option<Vec<String>>;
    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()>;
    fn key_to_str(&self, key: TagKey) -> Vec<&'static str>;
    fn get_tag(&self, key: TagKey) -> Vec<String> {
        let keystrs = self.key_to_str(key);
        if keystrs.is_empty() {
            debug!(
                "The {:?} key is not supported in the output format {:?}",
                key,
                self.format()
            );
            return vec![];
        }
        keystrs
            .into_iter()
            .filter_map(|keystr| self.get_str(keystr))
            .flatten()
            .collect()
    }
    fn set_tag(&mut self, key: TagKey, values: Vec<String>) -> Result<(), TagError> {
        let keystrs = self.key_to_str(key);
        if keystrs.is_empty() {
            return Err(TagError::NotSupported);
        }
        keystrs.into_iter().try_for_each(|keystr| {
            self.set_str(keystr, values.clone())
                .map_err(TagError::Other)
        })
    }

    fn write_to_path(&mut self, path: &Path) -> Result<()>;
}
