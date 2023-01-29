use chrono::NaiveDate;
use eyre::{eyre, Context, Result};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Duration;
use strfmt::strfmt;

use crate::track::key::TagKey;
use crate::util::path_to_str;

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";
pub const UNKNOWN_TITLE: &str = "(unkown title)";

pub trait GroupTracks {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)>;
}

pub trait Artists {
    fn names(&self) -> Vec<String>;
    fn ids(&self) -> Vec<String>;
    fn sort_order(&self) -> Vec<String>;
    fn joined(&self) -> String;
    fn sort_order_joined(&self) -> String;
    fn instruments(&self) -> Vec<String>;
}

impl Artists for Vec<Artist> {
    fn names(&self) -> Vec<String> {
        self.iter().map(|s| s.name.clone()).collect::<Vec<_>>()
    }
    fn ids(&self) -> Vec<String> {
        self.iter()
            .filter_map(|s| s.mbid.clone())
            .collect::<Vec<_>>()
    }
    fn sort_order(&self) -> Vec<String> {
        self.iter()
            .filter_map(|s| s.sort_name.clone())
            .collect::<Vec<_>>()
    }
    fn joined(&self) -> String {
        let mut res = "".to_string();
        for (i, artist) in self.iter().enumerate() {
            res.push_str(artist.name.as_str());
            if i >= self.len() - 1 {
                continue;
            }

            if let Some(join) = &artist.join_phrase {
                res.push_str(join.as_str());
            } else {
                // TODO: configuration
                res.push_str(", ");
            }
        }
        res
    }
    fn sort_order_joined(&self) -> String {
        let mut res = "".to_string();
        for (i, artist) in self.iter().enumerate() {
            if let Some(sort) = artist.sort_name.as_ref() {
                res.push_str(sort.as_str());
                if i >= self.len() - 1 {
                    continue;
                }

                if let Some(join) = &artist.join_phrase {
                    res.push_str(join.as_str());
                } else {
                    // TODO: configuration
                    res.push_str(", ");
                }
            }
        }
        res
    }
    fn instruments(&self) -> Vec<String> {
        self.iter()
            .flat_map(|s| {
                if !s.instruments.is_empty() {
                    s.instruments
                        .iter()
                        .map(|i| format!("{} ({})", s.name, i))
                        .collect()
                } else {
                    vec![s.name.clone()]
                }
            })
            .collect()
    }
}

pub trait Format {
    fn fmt(&self, template: &str) -> Result<String>;
}

impl Format for Artist {
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

impl Format for Track {
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

impl Format for Release {
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
