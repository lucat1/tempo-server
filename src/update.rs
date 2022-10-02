use crate::library::{Delete, Filter, LibraryTrack, Store};
use crate::models::{Format, Track};
use crate::util::mkdirp;
use eyre::{eyre, Result};
use log::{info, trace, warn};
use std::path::Path;
use std::time::Instant;

static FMT: &str = "{album_artist} - {track_title}";

pub async fn update(_filters: Vec<&String>) -> Result<()> {
    let start = Instant::now();
    let mut tracks = Track::filter::<String, String>(
        vec![],
        vec![" ORDER BY tracks.release, tracks.disc, tracks.number".to_string()],
    )
    .await?;
    for track in tracks.iter_mut() {
        trace!("Checking track {:?}", track);
        let mut updated = false;
        let path = track
            .path
            .as_ref()
            .ok_or(eyre!("Track {:?} has no path", track.mbid))?;
        if !Path::new(path.as_os_str()).exists() {
            warn!("Track \"{}\" has been deleted", track.fmt(FMT)?);
            track.delete().await?;
        }
        let new_path = track.path()?;
        if path != &new_path {
            warn!("Moving track \"{}\" to {:?}", track.fmt(FMT)?, new_path);
            if let Some(parent) = new_path.parent() {
                mkdirp(parent)?;
            }
            std::fs::rename(path, &new_path)?;
            track.path = Some(new_path);
            updated = true;
            // TODO move covers when album folders change
        }
        if updated {
            track.store().await?;
        }
    }
    info!("Done, took {:?}", start.elapsed());
    Ok(())
}
