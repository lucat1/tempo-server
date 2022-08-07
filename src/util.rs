use eyre::{eyre, Result};
use std::path::PathBuf;

pub fn path_to_str(path: &PathBuf) -> Result<String> {
    Ok(String::from(path.to_str().ok_or(eyre!(
        "Could not get track path as string: {:?}",
        path
    ))?))
}
