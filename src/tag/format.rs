use std::path::Path;
use eyre::{Result, WrapErr, eyre};

#[derive(Clone, Copy, Debug)]
pub enum Format {
    FLAC,
    M4A,
    ID3
}

impl Format {
    pub fn from_path<P>(path: P) -> Result<Format> where P: AsRef<Path> {
        Self::from_mime(
            infer::get_from_path(path)
                .wrap_err("Could not read file for magic number analysis")?
                .ok_or(eyre!("Could not identify file format from magic number"))?
                .mime_type()
        )
    }

    pub fn from_mime(mime: &str) -> Result<Format> {
        // Complete list here:
        // https://crates.io/crates/infer#audio
        match mime {
            "audio/mpeg" => Ok(Format::ID3),
            "audio/m4a" => Ok(Format::M4A),
            "audio/x-flac" => Ok(Format::FLAC),
            _ => Err(eyre!("Invalid file: either not an audio file or not a supported format:\n{:?}", mime))
        }
    }
}
