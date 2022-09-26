use chrono::NaiveDate;
use eyre::{eyre, Result};
use std::fs::create_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub fn path_to_str(path: &PathBuf) -> Result<String> {
    Ok(String::from(path.to_str().ok_or(eyre!(
        "Could not convert path to string: {:?}",
        path
    ))?))
}

pub fn dedup<T: Ord>(mut vec: Vec<T>) -> Vec<T> {
    vec.sort_unstable();
    vec.dedup();
    vec
}

pub fn mkdirp<P: AsRef<Path>>(path: &P) -> io::Result<()> {
    if let Err(e) = create_dir_all(path) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}

pub fn maybe_date(d: Option<String>) -> Option<NaiveDate> {
    d.map_or(None, |s| {
        NaiveDate::parse_from_str(s.as_str(), "%Y-%m-%d")
            .ok()
            .or(NaiveDate::parse_from_str(s.as_str(), "%Y").ok())
    })
}
