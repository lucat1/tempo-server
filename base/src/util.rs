use eyre::{eyre, Result};
use lazy_static::lazy_static;
use std::fs::create_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use time::{format_description::FormatItem, macros::format_description, parsing::Parsed};

pub fn path_to_str(path: &PathBuf) -> Result<String> {
    Ok(String::from(path.to_str().ok_or_else(|| {
        eyre!("Could not convert path to string: {:?}", path)
    })?))
}

pub fn dedup<T: Ord>(mut vec: Vec<T>) -> Vec<T> {
    vec.sort_unstable();
    vec.dedup();
    vec
}

pub fn mkdirp<P: AsRef<Path>>(path: P) -> io::Result<()> {
    if let Err(e) = create_dir_all(path) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}

lazy_static! {
    static ref DATE_FORMAT: &'static [FormatItem<'static>] =
        format_description!("[year][optional [-[month]]][optional [-[day]]]");
}

#[derive(Default, Debug, Copy, Clone)]
pub struct OptionalDate {
    pub year: Option<i32>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

pub fn maybe_date(d: Option<String>) -> OptionalDate {
    if let Some(s) = d {
        let mut parsed = Parsed::new();
        let parse_result = parsed.parse_items(s.as_bytes(), &DATE_FORMAT);
        let res = OptionalDate {
            year: parsed.year(),
            month: parsed.month().map(|m| m as u8),
            day: parsed.day().map(|d| d.into()),
        };
        tracing::trace!(date = %s, ?res, ?parse_result, "Parsed date");
        res
    } else {
        OptionalDate::default()
    }
}
