use entity::{ArtistActive, ReleaseActive, TrackActive};
use eyre::{eyre, Context, Result, WrapErr};
use setting::get_settings;
use std::collections::HashMap;
use std::path::PathBuf;
use strfmt::strfmt;

use crate::util::path_to_str;
use tag::TagKey;

pub trait Format {
    fn fmt(&self, template: &str) -> Result<String>;
}

impl Format for ArtistActive {
    fn fmt(&self, template: &str) -> Result<String> {
        let mut vars = HashMap::new();
        vars.insert("mbid".to_string(), self.mbid.clone().unwrap_or_default());
        vars.insert("name".to_string(), self.name.clone());
        vars.insert(
            "join_phrase".to_string(),
            self.join_phrase.clone().unwrap_or_default(),
        );
        vars.insert(
            "sort_name".to_string(),
            self.sort_name.clone().unwrap_or_default(),
        );
        vars.insert(
            "instruments".to_string(),
            self.instruments.join(", "), // TODO
        );
        strfmt(template, &vars)
            .map_err(|e| eyre!(e))
            .wrap_err(eyre!("Error while formatting artist string"))
    }
}

impl Format for TrackActive {
    fn fmt(&self, template: &str) -> Result<String> {
        let multiple_vars: HashMap<String, Vec<String>> = self.clone().try_into()?;
        let mut vars: HashMap<String, String> = multiple_vars
            .into_iter()
            .map(|(k, v)| (k, v.join(", "))) // TODO
            .collect();
        // Multiple value fields are different. The version held in the Track::artists
        // data structure also holds the information for mergining the various
        // artists into a single string. We therefore generate it from the
        // original track instance
        vars.insert(TagKey::Artist.to_string(), self.artists.joined());
        vars.insert(TagKey::Artists.to_string(), self.artists.joined());
        vars.insert(TagKey::OriginalArtist.to_string(), self.artists.joined());
        vars.insert(
            TagKey::ArtistSortOrder.to_string(),
            self.artists.sort_order_joined(),
        );
        vars.insert(
            TagKey::Genre.to_string(),
            self.genres.join(", "), // TODO
        );
        if let Some(release) = self.release.as_ref() {
            vars.insert(TagKey::AlbumArtist.to_string(), release.artists.joined());
            vars.insert(
                TagKey::AlbumArtistSortOrder.to_string(),
                release.artists.sort_order_joined(),
            );
        }
        if let Some(path) = self.path.as_ref() {
            vars.insert("path".to_string(), path_to_str(path)?);
        }
        if let Some(format) = self.format.as_ref() {
            vars.insert("format".to_string(), (*format).into());
        }
        strfmt(template, &vars)
            .map_err(|e| eyre!(e))
            .wrap_err(eyre!("Error while formatting track string"))
    }
}

impl Format for ReleaseActive {
    fn fmt(&self, template: &str) -> Result<String> {
        let multiple_vars: HashMap<String, Vec<String>> = self.clone().try_into()?;
        let mut vars: HashMap<String, String> = multiple_vars
            .into_iter()
            .map(|(k, v)| (k, v.join(", "))) // TODO
            .collect();
        // Multiple value fields are different. The version held in the Track::artists
        // data structure also holds the information for mergining the various
        // artists into a single string. We therefore generate it from the
        // original track instance
        vars.insert(TagKey::AlbumArtist.to_string(), self.artists.joined());
        vars.insert(
            TagKey::AlbumArtistSortOrder.to_string(),
            self.artists.sort_order_joined(),
        );
        strfmt(template, &vars)
            .map_err(|e| eyre!(e))
            .wrap_err(eyre!("Error while formatting release string"))
    }
}

pub trait InLibrary {
    fn filename(&self) -> Result<PathBuf>;
}

impl InLibrary for TrackActive {
    fn filename(&self) -> Result<PathBuf> {
        let settings =
            get_settings()?.wrap_err("While generating a track filename for the library")?;
        let mut builder = self.fmt(settings.track_name.as_str())?;
        builder.push('.');
        builder.push_str(
            self.format
                .ok_or(eyre!("The given Track doesn't have an associated format"))?
                .ext(),
        );
        Ok(builder)
    }
}

impl InLibrary for ReleaseActive {
    fn filename(&self) -> Result<PathBuf> {
        let settings =
            get_settings()?.wrap_err("While generating a release folder name for the library")?;
        Ok(settings
            .library
            .join(self.fmt(settings.release_name.as_str())?))
    }
}
