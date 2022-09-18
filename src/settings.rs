use directories::UserDirs;
use eyre::{eyre, Result};
use image::ImageOutputFormat;
use serde_derive::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub library: PathBuf,
    pub db: PathBuf,
    pub release_name: String,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtFormat {
    #[default]
    PNG,
    JPG,
}

impl ArtFormat {
    pub fn mime(&self) -> &'static str {
        match self {
            ArtFormat::PNG => "image/png",
            ArtFormat::JPG => "image/jpeg",
        }
    }
}

impl From<ArtFormat> for ImageOutputFormat {
    fn from(f: ArtFormat) -> Self {
        match f {
            ArtFormat::PNG => ImageOutputFormat::Png,
            ArtFormat::JPG => ImageOutputFormat::Jpeg(100),
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
}

impl Settings {
    pub fn gen_default() -> Result<Self> {
        let dirs = UserDirs::new().ok_or(eyre!("Could not locate user directories"))?;
        let library = dirs
            .audio_dir()
            .ok_or(eyre!("Could not locate current user's Audio directory"))?;

        Ok(Settings {
            library: library.to_path_buf(),
            db: library.join(PathBuf::from("db")),
            release_name: "{release.artist}/{release.title}".to_string(),
            track_name: "{track.disc} - {track.number} - {track.title}".to_string(),
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
