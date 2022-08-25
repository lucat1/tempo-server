use eyre::{bail, eyre, Result};
use std::path::PathBuf;

use crate::fetch::ArtistLike;

pub fn path_to_str(path: &PathBuf) -> Result<String> {
    Ok(String::from(path.to_str().ok_or(eyre!(
        "Could not get track path as string: {:?}",
        path
    ))?))
}

pub fn dedup<T: Ord>(mut vec: Vec<T>) -> Vec<T> {
    vec.sort_unstable();
    vec.dedup();
    vec
}

pub fn take_first<T: Clone>(v: Vec<T>, bail_msg: &'static str) -> Result<T> {
    if v.len() < 1 {
        bail!(bail_msg);
    }
    Ok(v[0].clone())
}

pub fn join_artists(artists: Vec<Box<dyn ArtistLike>>) -> String {
    let mut res = "".to_string();
    for (i, artist) in artists.iter().enumerate() {
        res.push_str(artist.name().as_str());
        if i >= artists.len() - 1 {
            continue;
        }

        if let Some(join) = artist.joinphrase() {
            res.push_str(join.as_str());
        }
    }
    res
}
