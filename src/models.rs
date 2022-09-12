use eyre::Result;
use sqlx::FromRow;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";

#[derive(Clone, Debug, FromRow)]
pub struct Artist {
    pub mbid: Option<String>,
    pub name: String,
    pub join_phrase: Option<String>,
    pub sort_name: Option<String>,
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
}

#[derive(Clone, Debug, FromRow)]
pub struct Release {
    pub mbid: Option<String>,
    pub title: String,
    pub artists: Vec<Artist>,
    pub discs: Option<u64>,
}

pub trait GroupTracks {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)>;
}

pub trait Artists {
    fn names(&self) -> Vec<String>;
    fn ids(&self) -> Vec<String>;
    fn sort_order(&self) -> String;
    fn joined(&self) -> String;
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
    fn sort_order(&self) -> String {
        // TODO: investigate on wether the artist.separator shall be used here
        self.iter()
            .filter_map(|s| s.sort_name.clone())
            .collect::<Vec<_>>()
            .join(",")
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
}
