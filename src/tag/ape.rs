extern crate ape;

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

impl crate::tag::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::tag::Tag>>
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

impl crate::tag::Tag for Tag {
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

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        ape::write_to_path(&self.tag, path).map_err(|e| eyre!(e))
    }
}
