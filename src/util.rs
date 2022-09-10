use eyre::{bail, eyre, Result};
use std::fs::create_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;

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

pub fn take_first<T: Clone>(v: Vec<T>, bail_msg: String) -> Result<T> {
    if v.len() < 1 {
        bail!(bail_msg);
    }
    Ok(v[0].clone())
}

pub fn mkdirp<P: AsRef<Path>>(path: &P) -> io::Result<()> {
    if let Err(e) = create_dir_all(path) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}
