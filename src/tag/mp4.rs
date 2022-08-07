extern crate mp4ameta;

use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use mp4ameta::ident::DataIdent;
use mp4ameta::{Data, ImgFmt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAGIC: u8 = 0xa9;

#[derive(Clone)]
pub struct Tag {
    tag: mp4ameta::Tag,
    separator: String,
}

impl crate::tag::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::tag::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: mp4ameta::Tag::read_from_path(path)?,
            // TODO
            separator: ",".to_string(),
        }))
    }
}

fn ident_to_string(ident: &DataIdent) -> String {
    match ident {
        DataIdent::Fourcc(d) => format!("{}", d),
        DataIdent::Freeform { mean, name } => format!("{}:{}", mean, name),
    }
}

fn str_to_ident(ident: &str) -> DataIdent {
    let mut bytes = ident.as_bytes().to_owned();
    // Replace UTF-8 © with the proper character
    if bytes.len() == 5 && bytes[0..2] == [194, 169] {
        bytes = vec![MAGIC, bytes[2], bytes[3], bytes[4]];
    }
    // Fourcc
    if bytes.len() == 4 {
        return DataIdent::fourcc(bytes.try_into().unwrap());
    }
    // Convert string freeform
    let mut ident = ident.replacen("----:", "", 1);
    // iTunes:VALUE abstraction
    if ident.starts_with("iTunes:") {
        ident = format!("com.apple.{}", ident);
    }
    let mut mean = "com.apple.iTunes";
    let mut name = ident.to_string();
    let split: Vec<&str> = ident.split(":").collect();
    if split.len() > 1 {
        mean = split[0];
        name = split[1].to_owned();
    }
    DataIdent::freeform(mean, name)
}

impl crate::tag::Tag for Tag {
    fn get_str(&self, key: &str) -> Option<Vec<String>> {
        let ident = str_to_ident(key);
        let data: Vec<String> = self
            .tag
            .data_of(&ident)
            .filter_map(|data| {
                // Save only text values
                match data {
                    Data::Utf8(d) => Some(d.to_owned()),
                    Data::Utf16(d) => Some(d.to_owned()),
                    _ => None,
                }
            })
            .collect();
        if data.is_empty() {
            return None;
        }
        // Convert multi tag to single with separator
        Some(
            data.join(&self.separator)
                .split(&self.separator)
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        )
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        self.tag
            .set_data(str_to_ident(key), Data::Utf8(values.join(&self.separator)));
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();

        for (ident, data) in self.tag.data() {
            let mut values = vec![];
            // Save only text values
            match data {
                Data::Utf8(d) => values = d.split(&self.separator).map(String::from).collect(),
                Data::Utf16(d) => values = d.split(&self.separator).map(String::from).collect(),
                _ => {}
            }
            if !values.is_empty() {
                out.insert(ident_to_string(ident), values);
            }
        }

        out
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        Ok(self
            .tag
            .images()
            .map(|img| Picture {
                mime_type: match img.1.fmt {
                    ImgFmt::Png => "image/png".to_string(),
                    ImgFmt::Jpeg => "image/jpeg".to_string(),
                    ImgFmt::Bmp => "image/bmp".to_string(),
                },
                picture_type: PictureType::CoverFront,
                description: ident_to_string(img.0),
                data: img.1.data.to_owned(),
            })
            .collect::<Vec<_>>())
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag.write_to_path(path).map_err(|e| eyre!(e))
    }
}
