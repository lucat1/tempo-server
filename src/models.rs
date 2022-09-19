use chrono::NaiveDate;
use eyre::Result;
use sqlx::FromRow;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

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
