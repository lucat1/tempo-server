use directories::{ProjectDirs, UserDirs};
use eyre::{eyre, Result};
use image::ImageOutputFormat;
use log::trace;
use mime::{Mime, IMAGE_JPEG, IMAGE_PNG};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;
use std::{fmt::Display, path::PathBuf};

use crate::{CLI_NAME, SETTINGS};

static DEFAULT_DB_FILE: &str = "lib.db";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub library: PathBuf,
    #[serde(default)]
    pub db: PathBuf,
    #[serde(default = "default_track_name")]
    pub track_name: String,

    #[serde(default)]
    pub tagging: Tagging,
    #[serde(default)]
    pub art: Art,
}

fn default_track_name() -> String {
    "{album_artist}/{album} ({release_year}) ({release_type})/{disc_number} - {track_number} - {track_title}"
                    .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tagging {
    #[serde(default = "default_true")]
    pub clear: bool,
    #[serde(default)]
    pub genre_limit: Option<usize>,
    #[serde(default = "default_true")]
    pub use_original_date: bool,

    #[serde(default = "default_id3_separator")]
    pub id3_separator: String,
    #[serde(default = "default_separator")]
    pub mp4_separator: String,
    #[serde(default = "default_separator")]
    pub ape_separator: String,
}

fn default_true() -> bool {
    true
}

fn default_id3_separator() -> String {
    "\0".to_string()
}

fn default_separator() -> String {
    ";".to_string()
}

impl Default for Tagging {
    fn default() -> Self {
        Self {
            clear: default_true(),
            genre_limit: Option::default(),
            use_original_date: default_true(),
            id3_separator: default_id3_separator(),
            mp4_separator: default_separator(),
            ape_separator: default_separator(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Art {
    #[serde(default = "default_art_providers")]
    pub providers: Vec<ArtProvider>,
    #[serde(default = "default_art_width")]
    pub width: u32,
    #[serde(default = "default_art_height")]
    pub height: u32,
    #[serde(default)]
    pub format: ArtFormat,
    #[serde(default = "default_art_image_name")]
    pub image_name: Option<String>,

    #[serde(default = "default_provider_relevance")]
    pub provider_relevance: f64,
    #[serde(default = "default_match_relevance")]
    pub match_relevance: f64,
    #[serde(default = "default_size_relevance")]
    pub size_relevance: f64,

    #[serde(default = "default_true")]
    pub cover_art_archive_use_release_group: bool,
}

fn default_art_providers() -> Vec<ArtProvider> {
    vec![ArtProvider::Itunes, ArtProvider::CoverArtArchive]
}

fn default_art_width() -> u32 {
    1200
}

fn default_art_height() -> u32 {
    1200
}

fn default_art_image_name() -> Option<String> {
    Some("cover".to_string())
}

fn default_provider_relevance() -> f64 {
    2.0 / 8.0
}

fn default_match_relevance() -> f64 {
    2.0 / 8.0
}

fn default_size_relevance() -> f64 {
    4.0 / 8.0
}

impl Default for Art {
    fn default() -> Self {
        Self {
            providers: default_art_providers(),
            width: default_art_width(),
            height: default_art_height(),
            format: ArtFormat::default(),
            image_name: default_art_image_name(),
            provider_relevance: default_provider_relevance(),
            match_relevance: default_match_relevance(),
            size_relevance: default_size_relevance(),
            cover_art_archive_use_release_group: default_true(),
        }
    }
}

fn get_library() -> Result<PathBuf> {
    UserDirs::new()
        .ok_or(eyre!("Could not locate user directories"))
        .and_then(|dirs| {
            dirs.audio_dir()
                .map(|audio| audio.to_path_buf())
                .ok_or(eyre!("Could not locate current user's Audio directory"))
        })
        .or_else(|_| PathBuf::from_str("/music").map_err(|e| eyre!(e)))
}

pub fn load() -> Result<Settings> {
    let dirs = ProjectDirs::from("com", "github", CLI_NAME)
        .ok_or(eyre!("Could not locate program directories"))?;
    let path = dirs.config_dir().join(PathBuf::from("config.toml"));
    trace!("Loading config file: {:?}", path);
    let content = fs::read_to_string(path).unwrap_or_else(|_| "".to_string());
    let mut set: Settings = toml::from_str(content.as_str()).map_err(|e| eyre!(e))?;
    let lib = get_library()?;
    if set.library == PathBuf::default() {
        set.library = lib.clone();
    }
    if set.db == PathBuf::default() {
        set.db = lib.join(DEFAULT_DB_FILE);
    }
    trace!("Loaded settings: {:?}", set);
    Ok(set)
}

pub fn print() -> Result<()> {
    let settings = SETTINGS.get().ok_or(eyre!("Could not read settings"))?;
    print!("{}", toml::to_string(settings)?);
    Ok(())
}
