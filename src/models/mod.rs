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
    pub number: Option<u64>,
    pub abs_number: Option<u64>,
    pub release: Option<Arc<Release>>,
}

#[derive(Clone, Debug, FromRow)]
pub struct Release {
    pub mbid: Option<String>,
    pub title: String,
    pub artists: Vec<Artist>,
}

pub trait GroupTracks {
    fn group_tracks(self) -> Result<(Release, Vec<Track>)>;
}

pub trait Joined {
    fn joined(&self) -> String;
}

impl Joined for Vec<Artist> {
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
