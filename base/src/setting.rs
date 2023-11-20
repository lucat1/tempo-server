use async_once_cell::OnceCell;
use directories::{ProjectDirs, UserDirs};
use eyre::{eyre, Result};
use image::ImageOutputFormat;
use lazy_static::lazy_static;
use mime::{Mime, IMAGE_GIF, IMAGE_JPEG, IMAGE_PNG};
use rand::distributions::{Alphanumeric, DistString};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::{fmt::Display, path::PathBuf};

use super::image_format::ImageFormat;
use super::{util, CLI_NAME};

lazy_static! {
    pub static ref SETTINGS: Arc<OnceCell<Settings>> = Arc::new(OnceCell::new());
}

static DEFAULT_DB_FILE: &str = "lib.db";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub db: String,
    #[serde(default = "default_taskie_url")]
    pub taskie: url::Url,
    #[serde(default = "default_url")]
    pub url: url::Url,
    #[serde(default)]
    pub library: Library,
    #[serde(default)]
    pub downloads: PathBuf,
    #[serde(default)]
    pub search_index: PathBuf,

    #[serde(default)]
    pub tasks: Tasks,

    #[serde(default)]
    pub connections: Connections,

    #[serde(default)]
    pub auth: Auth,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            db: String::new(),
            taskie: default_taskie_url(),
            url: default_url(),
            library: Library::default(),
            downloads: PathBuf::default(),
            search_index: PathBuf::default(),
            tasks: Tasks::default(),
            connections: Connections::default(),
            auth: Auth::default(),
        }
    }
}

fn default_taskie_url() -> url::Url {
    url::Url::parse("http://localhost:3000").unwrap()
}

fn default_url() -> url::Url {
    url::Url::parse("http://localhost:4000").unwrap()
}

fn default_library_name() -> String {
    "Main library".to_string()
}

fn default_artist_name() -> String {
    "{artist}".to_string()
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
    #[serde(default = "get_library")]
    pub path: PathBuf,
    #[serde(default = "default_artist_name")]
    pub artist_name: String,
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
            ImageFormat::Gif => IMAGE_GIF,
        }
    }
    pub fn from_mime(m: Mime) -> Option<Self> {
        match (m.type_(), m.subtype()) {
            (mime::IMAGE, mime::PNG) => Some(ImageFormat::Png),
            (mime::IMAGE, mime::JPEG) => Some(ImageFormat::Jpeg),
            (mime::IMAGE, mime::GIF) => Some(ImageFormat::Gif),
            (_, _) => None,
        }
    }
}

impl From<ImageFormat> for ImageOutputFormat {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Png => ImageOutputFormat::Png,
            ImageFormat::Jpeg => ImageOutputFormat::Jpeg(100),
            ImageFormat::Gif => ImageOutputFormat::Gif,
        }
    }
}

impl From<ImageFormat> for image::ImageFormat {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Png => image::ImageFormat::Png,
            ImageFormat::Jpeg => image::ImageFormat::Jpeg,
            ImageFormat::Gif => image::ImageFormat::Gif,
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
    pub provider_relevance: f32,
    #[serde(default = "default_match_relevance")]
    pub match_relevance: f32,
    #[serde(default = "default_size_relevance")]
    pub size_relevance: f32,

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

fn default_provider_relevance() -> f32 {
    2.0 / 8.0
}

fn default_match_relevance() -> f32 {
    2.0 / 8.0
}

fn default_size_relevance() -> f32 {
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

fn get_library() -> PathBuf {
    UserDirs::new()
        .and_then(|dirs| dirs.audio_dir().map(|audio| audio.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from_str("/music").unwrap())
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

fn get_search_index(path: &Path) -> PathBuf {
    path.join(".search")
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
    set = generate_default(set)?;
    tracing::trace! {settings = ?set,"Loaded settings"};
    Ok(set)
}

pub fn generate_default(mut set: Settings) -> Result<Settings> {
    if set.db == String::default() {
        set.db = format!(
            "sqlite://{}?mode=rwc",
            util::path_to_str(&set.library.path.join(DEFAULT_DB_FILE))?
        );
    }
    if set.downloads == PathBuf::default() {
        set.downloads = get_downloads()?;
    }
    if set.search_index == PathBuf::default() {
        set.search_index = get_search_index(&set.library.path);
    }
    if set.tasks.recurring == HashMap::default() {
        set.tasks.recurring = default_recurring();
    }
    if set.auth.jwt_secret == String::default() {
        set.auth.jwt_secret = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        tracing::warn!(secret = %set.auth.jwt_secret, "Using random JWT secret. Please define one in the config to make authentication persistant across restarts");
    }
    Ok(set)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tasks {
    #[serde(default = "num_cpus::get")]
    pub workers: usize,

    #[serde(default = "default_recurring")]
    pub recurring: HashMap<JobType, String>,

    #[serde(default = "default_outdated")]
    pub outdated: time::Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    ArtistUrl,
    ArtistDescription,
    LastFMArtistImage,
    IndexSearch,
}

fn default_outdated() -> time::Duration {
    time::Duration::DAY
}

fn default_recurring() -> HashMap<JobType, String> {
    [
        (JobType::ArtistUrl, "0 0 3 * * * *".to_string()),
        (JobType::ArtistDescription, "0 0 4 * * * *".to_string()),
        // (TaskType::ArtistImagesLastfm, "0 0 4 * * * *".to_string()),
    ]
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AuthMethod {
    Local,
    Ldap,
}

fn default_priority() -> Vec<AuthMethod> {
    vec![AuthMethod::Local]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auth {
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default = "default_priority")]
    pub priority: Vec<AuthMethod>,

    #[serde(default)]
    pub ldap: LDAP,

    #[serde(default)]
    pub users: Vec<User>,
}

impl Default for Auth {
    fn default() -> Self {
        Self {
            jwt_secret: String::new(),
            priority: default_priority(),

            ldap: LDAP::default(),
            users: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LDAP {
    #[serde(default = "default_ldap_uri")]
    pub uri: iref::IriBuf,
    #[serde(default = "default_ldap_base_dn")]
    pub base_dn: String,
    #[serde(default = "default_ldap_admin_pw")]
    pub admin_dn: String,
    #[serde(default = "default_ldap_admin_pw")]
    pub admin_pw: String,
    #[serde(default = "default_ldap_user_filter")]
    pub user_filter: String,
    #[serde(default)]
    pub attr_map: LdapAttrMap,
}

impl Default for LDAP {
    fn default() -> Self {
        Self {
            uri: default_ldap_uri(),
            base_dn: default_ldap_base_dn(),
            admin_dn: default_ldap_admin_dn(),
            admin_pw: default_ldap_admin_pw(),
            user_filter: default_ldap_user_filter(),
            attr_map: LdapAttrMap::default(),
        }
    }
}

fn default_ldap_uri() -> iref::IriBuf {
    iref::IriBuf::new("ldapi:///").unwrap()
}

fn default_ldap_base_dn() -> String {
    "dc=example,dc=com".to_string()
}

fn default_ldap_admin_dn() -> String {
    "cn=admin,dc=example,dc=com".to_string()
}

fn default_ldap_admin_pw() -> String {
    "admin_password".to_string()
}

fn default_ldap_user_filter() -> String {
    "(&(objectClass=inetOrgPerson)(uid={username}))".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LdapAttrMap {
    #[serde(default = "default_attr_username")]
    pub username: String,
    #[serde(default = "default_attr_first_name")]
    pub first_name: String,
    #[serde(default = "default_attr_last_name")]
    pub last_name: String,
}

fn default_attr_username() -> String {
    "uid".to_string()
}
fn default_attr_first_name() -> String {
    "givenName".to_string()
}
fn default_attr_last_name() -> String {
    "sn".to_string()
}

impl Default for LdapAttrMap {
    fn default() -> Self {
        Self {
            username: default_attr_username(),
            first_name: default_attr_first_name(),
            last_name: default_attr_last_name(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub struct Connections {
    pub lastfm: Option<LastFMConnection>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LastFMConnection {
    pub apikey: String,
    pub shared_secret: String,
}

pub fn get_settings() -> Result<&'static Settings> {
    SETTINGS.get().ok_or(eyre!("Could not get settings"))
}
