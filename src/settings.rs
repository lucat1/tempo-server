use directories::UserDirs;
use eyre::{eyre, Result};
use image::ImageOutputFormat;
use mime::{Mime, IMAGE_JPEG, IMAGE_PNG};
use serde_derive::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::{fmt::Display, path::PathBuf};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub library: PathBuf,
    pub db: PathBuf,
    pub track_name: String,

    pub tagging: Tagging,
    pub art: Art,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tagging {
    pub clear: bool,
    pub genre_limit: Option<usize>,
    pub use_original_date: bool,
    pub use_release_group: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtProvider {
    CoverArtArchive,
    Itunes,
}

impl Display for ArtProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtProvider::CoverArtArchive => write!(f, "CoverArtArchive"),
            ArtProvider::Itunes => write!(f, "iTunes"),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtFormat {
    Png,
    #[default]
    Jpeg,
}

impl ArtFormat {
    pub fn mime(&self) -> Mime {
        match self {
            ArtFormat::Png => IMAGE_PNG,
            ArtFormat::Jpeg => IMAGE_JPEG,
        }
    }
}

impl From<ArtFormat> for ImageOutputFormat {
    fn from(f: ArtFormat) -> Self {
        match f {
            ArtFormat::Png => ImageOutputFormat::Png,
            ArtFormat::Jpeg => ImageOutputFormat::Jpeg(100),
        }
    }
}

#[derive(SmartDefault, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Art {
    #[default(_code = "vec![ArtProvider::Itunes, ArtProvider::CoverArtArchive]")]
    pub providers: Vec<ArtProvider>,
    #[default = 1200]
    pub width: u32,
    #[default = 1200]
    pub height: u32,
    pub format: ArtFormat,
    #[default(_code = "Some(\"cover\".to_string())")]
    pub filename: Option<String>,
}

impl Settings {
    pub fn gen_default() -> Result<Self> {
        let dirs = UserDirs::new().ok_or(eyre!("Could not locate user directories"))?;
        let library = dirs
            .audio_dir()
            .ok_or(eyre!("Could not locate current user's Audio directory"))?;

        Ok(Settings {
            library: library.to_path_buf(),
            db: library.join(PathBuf::from("lib.db")),
            track_name:
                "{album_artist}/{album} ({release_year}) ({release_type})/{disc_number} - {track_number} - {track_title}"
                    .to_string(),
            tagging: Tagging {
                clear: true,
                genre_limit: None,
                use_original_date: true,
                use_release_group: true,
            },
            art: Art::default(),
        })
    }
}
