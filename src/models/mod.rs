use crate::fetch::Fetch;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Artist {
    pub mbid: Option<String>,
    pub name: String,
    pub join_phrase: Option<String>,
    pub sort_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub mbid: Option<String>,
    pub title: String,
    pub artists: Vec<Artist>,
    pub length: Option<Duration>,
    pub disc: Option<u64>,
    pub number: Option<u64>,
    pub album: Option<Box<Release>>,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub fetcher: Option<Arc<dyn Fetch + Send + Sync>>,
    pub mbid: Option<String>,
    pub title: String,
    pub artists: Vec<Artist>,
    pub tracks: Vec<Track>,
}

impl Release {
    pub fn artists_joined(&self) -> String {
        let mut res = "".to_string();
        for (i, artist) in self.artists.iter().enumerate() {
            res.push_str(artist.name.as_str());
            if i >= self.artists.len() - 1 {
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
