use crate::models::{Release, Track};
use crate::SETTINGS;
use eyre::{eyre, Result, WrapErr};
use std::path::PathBuf;

pub trait LibraryRelease {
    fn paths(&self) -> Result<Vec<PathBuf>>;
    fn path(&self) -> Result<PathBuf>;
    fn other_paths(&self) -> Result<Vec<PathBuf>>;
}

impl LibraryRelease for Release {
    fn paths(&self) -> Result<Vec<PathBuf>> {
        let mut v = vec![];
        let settings = SETTINGS
            .get()
            .ok_or(eyre!("Could not read settings"))
            .wrap_err("While generating a path for the library")?;
        for artist in self.artists.iter() {
            let path_str = settings
                .release_name
                .replace("{release.artist}", artist.name.as_str())
                .replace("{release.title}", self.title.as_str());
            v.push(settings.library.join(PathBuf::from(path_str)))
        }
        Ok(v)
    }

    fn path(&self) -> Result<PathBuf> {
        self.paths()?
            .first()
            .map_or(
                Err(eyre!("Release does not have a path in the library, most definitely because the release has no artists")),
                |p| Ok(p.clone())
            )
    }

    fn other_paths(&self) -> Result<Vec<PathBuf>> {
        let main = self.path()?;
        Ok(self
            .paths()?
            .iter()
            .filter_map(|p| -> Option<PathBuf> {
                if *p != main {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }
}

pub trait LibraryTrack {
    fn path(&self, ext: &str) -> Result<PathBuf>;
}

impl LibraryTrack for Track {
    fn path(&self, ext: &str) -> Result<PathBuf> {
        let base = self
            .release
            .clone()
            .ok_or(eyre!("This track doesn't belong to any release"))?
            .path()?;
        let settings = SETTINGS
            .get()
            .ok_or(eyre!("Could not read settings"))
            .wrap_err("While generating a path for the library")?;
        let mut extensionless = settings.track_name.clone();
        extensionless.push('.');
        extensionless.push_str(ext);
        let path_str = extensionless
            .replace(
                "{track.disc}",
                self.disc
                    .ok_or(eyre!("The track has no disc"))?
                    .to_string()
                    .as_str(),
            )
            .replace(
                "{track.number}",
                self.number
                    .ok_or(eyre!("The track has no number"))?
                    .to_string()
                    .as_str(),
            )
            .replace("{track.title}", self.title.as_str());
        Ok(base.join(path_str))
    }
}
