use crate::models::{Artist, Artists};
use crate::{library::Filter, models::Track};
use eyre::{eyre, Result, WrapErr};
use log::{info, warn};
use regex::Regex;
use std::time::Instant;

pub async fn list(filters: Vec<&String>) -> Result<()> {
    let start = Instant::now();
    let tracks = Track::filter::<String, String>(
        vec![],
        vec![" ORDER BY release, disc, number".to_string()],
    )
    .await?;
    for track in tracks.into_iter() {
        println!(
            "{} - {} ({}-{})",
            track.artists.joined(),
            track.title,
            track.disc.unwrap_or(0),
            track.number.unwrap_or(0)
        )
    }
    info!("Took {:?}", start.elapsed());
    Ok(())
}
