use base::setting::{ArtProvider, Library};
use entity::full::FullRelease;
use eyre::{bail, eyre, Result};
use image::imageops::{resize, FilterType};
use image::DynamicImage;
use image::{io::Reader as ImageReader, ImageOutputFormat};
use mime::Mime;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Cursor;
use std::time::Instant;

use super::CLIENT;
use super::{cover_art_archive, deezer, itunes};

pub async fn search(library: &Library, release: &FullRelease) -> Result<Vec<Vec<Cover>>> {}
