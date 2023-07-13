mod cover_art_archive;
mod deezer;
mod itunes;
mod music_brainz;

// pub mod cover;
// pub use cover::Cover;
// pub use music_brainz::ReleaseSearch;

use const_format::formatcp;
use eyre::{bail, eyre, Context, Result};
use lazy_static::lazy_static;
use reqwest::header::USER_AGENT;
use serde::Serialize;
use std::time::Instant;
