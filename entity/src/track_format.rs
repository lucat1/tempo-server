use eyre::{eyre, Report, Result, WrapErr};
use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i8", db_type = "Integer")]
pub enum TrackFormat {
    #[sea_orm(num_value = 0)]
    Flac,
    #[sea_orm(num_value = 1)]
    Mp4,
    #[sea_orm(num_value = 2)]
    Id3,
    #[sea_orm(num_value = 3)]
    Ape,
}

impl TrackFormat {
    pub fn from_path<P>(path: P) -> Result<TrackFormat>
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

    pub fn from_ext(ext: &str) -> Result<TrackFormat> {
        match ext {
            "flac" => Ok(TrackFormat::Flac),
            "mp4" => Ok(TrackFormat::Mp4),
            "id3" => Ok(TrackFormat::Id3),
            "ape" => Ok(TrackFormat::Ape),
            _ => Err(eyre!("Unkown extension format with extension {}", ext)),
        }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            TrackFormat::Flac => "flac",
            TrackFormat::Mp4 => "mp4",
            TrackFormat::Id3 => "mp3",
            TrackFormat::Ape => "ape",
        }
    }

    pub fn from_mime(mime: &str) -> Result<TrackFormat> {
        // Complete list here:
        // https://crates.io/crates/infer#audio
        match mime {
            "audio/mpeg" => Ok(TrackFormat::Id3),
            "audio/m4a" => Ok(TrackFormat::Mp4),
            "audio/x-flac" => Ok(TrackFormat::Flac),
            "audio/x-ape" => Ok(TrackFormat::Ape),
            _ => Err(eyre!(
                "Invalid file: either not an audio file or not a supported format:\n{:?}",
                mime
            )),
        }
    }

    pub fn mime(&self) -> String {
        match self {
            TrackFormat::Id3 => "audio/mpeg",
            TrackFormat::Mp4 => "audio/m4a",
            TrackFormat::Flac => "audio/x-flac",
            TrackFormat::Ape => "audio/x-ape",
        }
        .to_string()
    }
}

impl TryFrom<String> for TrackFormat {
    type Error = Report;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "flac" => Ok(TrackFormat::Flac),
            "mp4" => Ok(TrackFormat::Mp4),
            "id3" => Ok(TrackFormat::Id3),
            "ape" => Ok(TrackFormat::Ape),
            _ => Err(eyre!("Invalid format: {}", s)),
        }
    }
}

impl From<TrackFormat> for String {
    fn from(f: TrackFormat) -> Self {
        match f {
            TrackFormat::Flac => "flac".to_string(),
            TrackFormat::Mp4 => "mp4".to_string(),
            TrackFormat::Id3 => "id3".to_string(),
            TrackFormat::Ape => "ape".to_string(),
        }
    }
}
