use crate::models::{Format, Track};
use crate::SETTINGS;
use eyre::{eyre, Result, WrapErr};
use std::path::PathBuf;
use std::str::FromStr;

pub trait LibraryTrack {
    fn path(&self) -> Result<PathBuf>;
}

impl LibraryTrack for Track {
    fn path(&self) -> Result<PathBuf> {
        let settings = SETTINGS
            .get()
            .ok_or(eyre!("Could not read settings"))
            .wrap_err("While generating a path for the library")?;
        let mut builder = self.fmt(settings.track_name.as_str())?;
        builder.push('.');
        builder.push_str(
            self.format
                .ok_or(eyre!("The given Track doesn't have an associated format"))?
                .ext(),
        );
        Ok(settings
            .library
            .join(PathBuf::from_str(builder.as_str()).map_err(|e| eyre!(e))?))
    }
}
