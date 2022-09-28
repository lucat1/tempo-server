use crate::SETTINGS;
use eyre::eyre;
use eyre::{bail, Result};
use mime::Mime;
use std::fs::write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureType {
    Other,
    Icon,
    OtherIcon,
    CoverFront,
    CoverBack,
    Leaflet,
    Media,
    LeadArtist,
    Artist,
    Conductor,
    Band,
    Composer,
    Lyricist,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    ScreenCapture,
    BrightFish,
    Illustration,
    BandLogo,
    PublisherLogo,
}

#[derive(Clone)]
pub struct Picture {
    pub mime_type: Mime,
    pub picture_type: PictureType,
    pub description: String,
    pub data: Vec<u8>,
}

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Picture")
            .field("mime_type", &self.mime_type)
            .field("picture_type", &self.picture_type)
            .field("description", &self.description)
            .finish()
    }
}

pub fn write_picture<P>(picture: &Picture, root: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let cover_name = &SETTINGS
        .get()
        .ok_or(eyre!("Could not read settings"))?
        .art
        .filename;
    let name = match cover_name {
        Some(n) => n.to_string(),
        None => bail!("Picture write not required"),
    };
    let ext = picture.mime_type.subtype().as_str();
    let filename = PathBuf::from_str((name + "." + ext).as_str())?;
    let path = root.as_ref().join(filename);
    write(path, &picture.data).map_err(|e| eyre!(e))
}
