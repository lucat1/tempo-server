extern crate metaflac;

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use core::convert::AsRef;
use eyre::Result;

#[derive(Clone)]
pub struct Tag {
    tag: metaflac::Tag
}

impl crate::tag::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::tag::Tag>>
        where P: AsRef<Path> {
        Ok(Box::new(Tag{
            tag: metaflac::Tag::read_from_path(path)?
        }))
    }
}

impl crate::tag::Tag for Tag {
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        if let Some(values) = self.tag.get_vorbis(tag) {
            let v: Vec<&str> = values.collect();
            if v.is_empty() {
                return None;
            }

            return Some(v.into_iter().map(|v| v.to_string()).collect());
        }
        None
    }

    fn get_all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        if let Some(vorbis) = self.tag.vorbis_comments() {
            // Get value of tag with proper separators
            vorbis.comments.iter().for_each(|(k, _)| { out.insert(k.to_string(), self.get_raw(k).unwrap()); } );
        }
        out
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag.write_to_path(path)?;
        Ok(())
    }
}
