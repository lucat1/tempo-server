use async_once_cell::OnceCell;
use directories::{ProjectDirs, UserDirs};
use eyre::{eyre, Result};
use image::ImageOutputFormat;
use lazy_static::lazy_static;
use mime::{Mime, IMAGE_JPEG, IMAGE_PNG};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use std::{fmt::Display, path::PathBuf};

use super::image_format::ImageFormat;
use super::util;

lazy_static! {
    pub static ref SETTINGS: Arc<OnceCell<Settings>> = Arc::new(OnceCell::new());
}

const CLI_NAME: &str = "tagger";
static DEFAULT_DB_FILE: &str = "lib.db";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub db: String,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub downloads: PathBuf,
}

fn default_library_name() -> String {
    "Main library".to_string()
}

fn default_release_name() -> String {
    "{album_artist}/{album} ({release_year}) ({release_type})".to_string()
}

fn default_track_name() -> String {
    "{disc_number} - {track_number} - {track_title}".to_string()
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Library {
    #[serde(default = "default_library_name")]
    pub name: String,
    #[serde(default)]
    pub path: PathBuf,
    #[serde(default = "default_release_name")]
    pub release_name: String,
    #[serde(default = "default_track_name")]
    pub track_name: String,

    #[serde(default)]
    pub tagging: Tagging,
    #[serde(default)]
    pub art: Art,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tagging {
    #[serde(default = "default_true")]
    pub clear: bool,
    #[serde(default)]
    pub genre_limit: Option<usize>, // TODO: reimplement genre limits
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ArtProvider {
    CoverArtArchive,
    Itunes,
    Deezer,
}

impl Display for ArtProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtProvider::CoverArtArchive => write!(f, "CoverArtArchive"),
            ArtProvider::Itunes => write!(f, "iTunes"),
            ArtProvider::Deezer => write!(f, "Deezer"),
        }
    }
}

impl ImageFormat {
    pub fn mime(&self) -> Mime {
        match self {
            ImageFormat::Png => IMAGE_PNG,
            ImageFormat::Jpeg => IMAGE_JPEG,
        }
    }
}

impl From<ImageFormat> for ImageOutputFormat {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Png => ImageOutputFormat::Png,
            ImageFormat::Jpeg => ImageOutputFormat::Jpeg(100),
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
    pub format: ImageFormat,
    #[serde(default = "default_art_image_name")]
    pub image_name: String,

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
    vec![
        ArtProvider::Itunes,
        ArtProvider::Deezer,
        ArtProvider::CoverArtArchive,
    ]
}

fn default_art_width() -> u32 {
    1200
}

fn default_art_height() -> u32 {
    1200
}

fn default_art_image_name() -> String {
    "cover".to_string()
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
            format: ImageFormat::default(),
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

fn get_downloads() -> Result<PathBuf> {
    UserDirs::new()
        .ok_or(eyre!("Could not locate user directories"))
        .and_then(|dirs| {
            dirs.download_dir()
                .map(|audio| audio.to_path_buf())
                .ok_or(eyre!("Could not locate current user's Downloads directory"))
        })
        .or_else(|_| PathBuf::from_str("/downloads").map_err(|e| eyre!(e)))
}

pub fn load(path: Option<PathBuf>) -> Result<Settings> {
    let path = path.unwrap_or({
        let dirs = ProjectDirs::from("com", "github", CLI_NAME)
            .ok_or(eyre!("Could not locate program directories"))?;
        dirs.config_dir().join(PathBuf::from("config.toml"))
    });
    tracing::info! {?path, "Loading config file"};
    let content = fs::read_to_string(path).unwrap_or_else(|_| "".to_string());
    let mut set: Settings = toml::from_str(content.as_str()).map_err(|e| eyre!(e))?;
    if set.libraries.is_empty() {
        set.libraries.push(Library {
            name: default_library_name(),
            path: get_library()?,
            release_name: default_release_name(),
            track_name: default_track_name(),
            ..Default::default()
        });
    }
    if set.db == String::default() {
        let lib = set
            .libraries
            .first()
            .ok_or(eyre!("No libraries have been defined"))?;
        set.db = format!(
            "sqlite://{}?mode=rwc",
            util::path_to_str(&lib.path.join(DEFAULT_DB_FILE))?
        );
    }
    if set.downloads == PathBuf::default() {
        set.downloads = get_downloads()?;
    }
    tracing::trace! {settings = ?set,"Loaded settings"};
    Ok(set)
}

pub fn get_settings() -> Result<&'static Settings> {
    SETTINGS.get().ok_or(eyre!("Could not get settings"))
}

pub fn print() -> Result<()> {
    let settings = get_settings()?;
    print!("{}", toml::to_string(settings)?);
    Ok(())
}
