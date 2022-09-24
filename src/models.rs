use chrono::Datelike;
use chrono::NaiveDate;
use eyre::{eyre, Result};
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use strfmt::strfmt;

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";
pub const UNKNOWN_TITLE: &str = "(unkown title)";

#[derive(Clone, Debug, FromRow)]
pub struct Artist {
    pub mbid: Option<String>,
    pub name: String,
    pub join_phrase: Option<String>,
    pub sort_name: Option<String>,
    pub instruments: Vec<String>,
}

#[derive(Clone, Debug, FromRow)]
pub struct Track {
    pub mbid: Option<String>,
    pub title: String,
    pub artists: Vec<Artist>,
    pub length: Option<Duration>,
    pub disc: Option<u64>,
    pub disc_mbid: Option<String>,
    // TODO: discids, consider referencing a medium as well as the release
    // would include things like numbering and disc data in a more appropriate
    // structure. Con: would increase memory management complexity
    pub number: Option<u64>,
    pub genres: Vec<String>,
    pub release: Option<Arc<Release>>,

    // Performer, Vocal, Instrument
    pub performers: Vec<Artist>,
    pub engigneers: Vec<Artist>,
    pub mixers: Vec<Artist>,
    pub producers: Vec<Artist>,
    pub lyricists: Vec<Artist>,
    pub writers: Vec<Artist>,
    pub composers: Vec<Artist>,
}

#[derive(Clone, Debug, FromRow)]
pub struct Release {
    pub mbid: Option<String>,
    pub release_group_mbid: Option<String>,
    pub asin: Option<String>,
    pub title: String,
    pub artists: Vec<Artist>,
    pub discs: Option<u64>,
    pub media: Option<String>,
    pub tracks: Option<u64>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub catalog_no: Option<String>,
    pub status: Option<String>,
    pub release_type: Option<String>,
    pub date: Option<NaiveDate>,
    pub original_date: Option<NaiveDate>,
    pub script: Option<String>,
}

pub trait GroupTracks {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)>;
}

pub trait Artists {
    fn names(&self) -> Vec<String>;
    fn ids(&self) -> Vec<String>;
    fn sort_order(&self) -> Vec<String>;
    fn joined(&self) -> String;
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
        for (i, artist) in self.into_iter().enumerate() {
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
    fn instruments(&self) -> Vec<String> {
        self.iter()
            .map(|s| {
                if s.instruments.len() > 0 {
                    s.instruments
                        .iter()
                        .map(|i| format!("{} ({})", s.name, i))
                        .collect()
                } else {
                    vec![s.name.clone()]
                }
            })
            .flatten()
            .collect()
    }
}

pub trait Format {
    fn fmt(&self, template: &str) -> Result<String>;
}

impl Format for Artist {
    fn fmt(&self, template: &str) -> Result<String> {
        let mut vars = HashMap::new();
        vars.insert(
            "mbid".to_string(),
            self.mbid.clone().unwrap_or("".to_string()),
        );
        vars.insert("name".to_string(), self.name.clone());
        vars.insert(
            "join_phrase".to_string(),
            self.join_phrase.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "sort_name".to_string(),
            self.sort_name.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "instruments".to_string(),
            self.instruments.join(", "), // TODO
        );
        strfmt(template, &vars).map_err(|e| eyre!(e))
    }
}

impl Format for Track {
    fn fmt(&self, template: &str) -> Result<String> {
        let mut vars = HashMap::new();
        vars.insert(
            "mbid".to_string(),
            self.mbid.clone().unwrap_or("".to_string()),
        );
        vars.insert("title".to_string(), self.title.clone());
        vars.insert("artists".to_string(), self.artists.joined());
        vars.insert(
            "artists_sort".to_string(),
            self.artists.sort_order().join(", "),
        ); // TODO
        vars.insert(
            "length".to_string(),
            self.length
                .map_or("0".to_string(), |t| t.as_secs().to_string()),
        );
        vars.insert(
            "disc".to_string(),
            self.disc.map_or("0".to_string(), |d| d.to_string()),
        );
        vars.insert(
            "disc_mbid".to_string(),
            self.disc_mbid.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "number".to_string(),
            self.number.map_or("0".to_string(), |d| d.to_string()),
        );
        vars.insert(
            "genres".to_string(),
            self.genres.join(", "), // TODO
        );
        vars.insert(
            "performers".to_string(),
            self.performers.instruments().join(", "), // TODO
        );
        vars.insert("engigneers".to_string(), self.engigneers.joined());
        vars.insert("mixers".to_string(), self.mixers.joined());
        vars.insert("producers".to_string(), self.producers.joined());
        vars.insert("lyricists".to_string(), self.lyricists.joined());
        vars.insert("writers".to_string(), self.writers.joined());
        vars.insert("composers".to_string(), self.composers.joined());
        strfmt(template, &vars).map_err(|e| eyre!(e))
    }
}

impl Format for Release {
    fn fmt(&self, template: &str) -> Result<String> {
        let mut vars = HashMap::new();
        vars.insert(
            "mbid".to_string(),
            self.mbid.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "release_group_mbid".to_string(),
            self.mbid.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "asin".to_string(),
            self.release_group_mbid.clone().unwrap_or("".to_string()),
        );
        vars.insert("title".to_string(), self.title.clone());
        vars.insert("artists".to_string(), self.artists.joined());
        vars.insert(
            "artists_sort".to_string(),
            self.artists.sort_order().join(", "),
        ); // TODO
        vars.insert(
            "discs".to_string(),
            self.discs.map_or("0".to_string(), |d| d.to_string()),
        );
        vars.insert(
            "media".to_string(),
            self.media.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "tracks".to_string(),
            self.tracks.map_or("0".to_string(), |d| d.to_string()),
        );
        vars.insert(
            "country".to_string(),
            self.country.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "label".to_string(),
            self.label.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "catalog_no".to_string(),
            self.catalog_no.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "status".to_string(),
            self.status.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "release_type".to_string(),
            self.release_type.clone().unwrap_or("".to_string()),
        );
        vars.insert(
            "date".to_string(),
            self.date.map_or("".to_string(), |d| d.to_string()),
        );
        vars.insert(
            "year".to_string(),
            self.date.map_or("".to_string(), |d| d.year().to_string()),
        );
        vars.insert(
            "original_date".to_string(),
            self.original_date.map_or("".to_string(), |d| d.to_string()),
        );
        vars.insert(
            "original_year".to_string(),
            self.original_date
                .map_or("".to_string(), |d| d.year().to_string()),
        );
        vars.insert(
            "script".to_string(),
            self.script.clone().unwrap_or("".to_string()),
        );
        strfmt(template, &vars).map_err(|e| eyre!(e))
    }
}
