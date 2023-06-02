use eyre::{eyre, Result};
use lazy_static::lazy_static;
use std::fs::create_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};

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
    static ref FORMAT_YEAR: &'static [FormatItem<'static>] = format_description!("[year]");
    static ref FORMAT_YEAR_MONTH: &'static [FormatItem<'static>] =
        format_description!("[month]-[year]");
    static ref FORMAT_YEAR_MONTH_DAY: &'static [FormatItem<'static>] =
        format_description!("[month]-[year]-[day]");
}

pub struct OptionalDate {
    pub year: Option<i32>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

pub fn maybe_date(d: Option<String>) -> OptionalDate {
    let mut year = None;
    let mut month = None;
    let mut day = None;
    if let Some(s) = d {
        if let Ok(year_date) = OffsetDateTime::parse(s.as_str(), &FORMAT_YEAR) {
            year = Some(year_date.year());
        } else if let Ok(year_month_date) = OffsetDateTime::parse(s.as_str(), &FORMAT_YEAR_MONTH) {
            year = Some(year_month_date.year());
            month = Some(year_month_date.month() as u8);
        } else if let Ok(year_month_day_date) =
            OffsetDateTime::parse(s.as_str(), &FORMAT_YEAR_MONTH_DAY)
        {
            year = Some(year_month_day_date.year());
            month = Some(year_month_day_date.month() as u8);
            day = Some(year_month_day_date.day());
        }
    }
    OptionalDate { year, month, day }
}
