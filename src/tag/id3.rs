extern crate id3;

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use core::convert::AsRef;
use id3::{Content, TagLike, Version};
use eyre::Result;

#[derive(Clone)]
pub struct Tag {
    tag: id3::Tag,
    separator: String
}

impl crate::tag::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::tag::Tag>>
        where P: AsRef<Path> {
        Ok(Box::new(Tag{
            tag: id3::Tag::read_from_path(path)?,
            // TODO
            separator: ",".to_string()
        }))
    }
}

impl crate::tag::Tag for Tag {
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        // Custom tag (TXXX)
        if tag.len() != 4 {
            if let Some(t) = self.tag.extended_texts().find(|t| t.description == tag) {
                return Some(vec![t.value.to_string()]);
            }
            return None;
        }

        // Get tag
        if let Some(t) = self.tag.get(tag) {
            if let Some(content) = t.content().text() {
                Some(content.split(&self.separator).map(String::from).collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut tags = HashMap::new();
        for frame in self.tag.frames() {
            if let Content::Text(v) = frame.content() {
                tags.insert(frame.id().to_owned(), v.split(&self.separator).map(String::from).collect());
            }
        }
        // Add TXXX
        for extended in self.tag.extended_texts() {
            tags.insert(extended.description.to_string(), extended.value.split(&self.separator).map(String::from).collect());
        }
        tags
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag.write_to_path(path, Version::Id3v24)?;
        Ok(())
    }
}
