use eyre::{eyre, Report, Result, WrapErr};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub enum Format {
    FLAC,
    MP4,
    ID3,
    APE,
}

impl Format {
    pub fn from_path<P>(path: P) -> Result<Format>
    where
        P: AsRef<Path>,
    {
        Self::from_mime(
            infer::get_from_path(path)
                .wrap_err("Could not read file for magic number analysis")?
                .ok_or(eyre!("Could not identify file format from magic number"))?
                .mime_type(),
        )
    }

    pub fn from_mime(mime: &str) -> Result<Format> {
        // Complete list here:
        // https://crates.io/crates/infer#audio
        match mime {
            "audio/mpeg" => Ok(Format::ID3),
            "audio/m4a" => Ok(Format::MP4),
            "audio/x-flac" => Ok(Format::FLAC),
            "audio/x-ape" => Ok(Format::APE),
            _ => Err(eyre!(
                "Invalid file: either not an audio file or not a supported format:\n{:?}",
                mime
            )),
        }
    }

    pub fn from_ext(ext: &str) -> Result<Format> {
        match ext {
            "flac" => Ok(Format::FLAC),
            "mp4" => Ok(Format::MP4),
            "mp3" => Ok(Format::ID3),
            "ape" => Ok(Format::APE),
            _ => Err(eyre!("Unkown extension format with extension {}", ext)),
        }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            Format::FLAC => "flac",
            Format::MP4 => "mp4",
            Format::ID3 => "mp3",
            Format::APE => "ape",
        }
    }
}

impl TryFrom<String> for Format {
    type Error = Report;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "flac" => Ok(Format::FLAC),
            "mp4" => Ok(Format::MP4),
            "id3" => Ok(Format::ID3),
            "ape" => Ok(Format::APE),
            _ => Err(eyre!("Invalid format: {}", s)),
        }
    }
}

impl From<Format> for String {
    fn from(f: Format) -> Self {
        match f {
            Format::FLAC => "flac".to_string(),
            Format::MP4 => "mp4".to_string(),
            Format::ID3 => "id3".to_string(),
            Format::APE => "ape".to_string(),
        }
    }
}
