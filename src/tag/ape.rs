extern crate ape;

use ape::ItemValue;
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
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        let item = self.tag.item(tag)?;
        value_to_strings(&item.value, &self.separator)
    }

    fn get_all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        for item in self.tag.iter() {
            if let Some(vals) = value_to_strings(&item.value, &self.separator) {
                out.insert(item.key.clone(), vals);
            }
        }
        out
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        ape::write_to_path(&self.tag, path).map_err(|e| eyre!(e))
    }
}
