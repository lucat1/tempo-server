use directories::UserDirs;
use eyre::{eyre, Result};
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub library: PathBuf,
    pub db: PathBuf,
    pub release_name: String,
    pub track_name: String,

    pub tagging: Tagging,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tagging {
    pub clear: bool,
    pub use_original_date: bool,
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
                use_original_date: true,
            },
        })
    }
}
