use crate::fetch::get;
use crate::library::{LibraryTrack, Store};
use anyhow::Error;
use eyre::Result;
use indicatif::ProgressBar;
use log::{info, warn};
use sqlx::{query_as, Executor, FromRow, Sqlite};
use sqlx_migrate::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

static MUTATE: &str = "
DROP TABLE release_artists;
DROP TABLE track_artists;
DROP TABLE track_performers;
DROP TABLE track_engigneers;
DROP TABLE track_mixers;
DROP TABLE track_producers;
DROP TABLE track_lyricists;
DROP TABLE track_writers;
DROP TABLE track_composers;
DROP TABLE artists;
DROP TABLE tracks;
DROP TABLE releases;

CREATE TABLE IF NOT EXISTS artists (
  mbid BLOB PRIMARY KEY,
  name TEXT NOT NULL,
  sort_name TEXT,
  instruments TEXT
);
CREATE TABLE IF NOT EXISTS artist_credits (
  id integer PRIMARY KEY,
  artist blob NOT NULL,
  join_phrase varchar(256),

  UNIQUE(artist, join_phrase),
  FOREIGN KEY(artist) REFERENCES artists(mbid)
);
CREATE TABLE IF NOT EXISTS releases (
  mbid BLOB PRIMARY KEY,
  release_group_mbid BLOB,
  asin TEXT,
  title TEXT NOT NULL,
  discs NUMBER,
  media TEXT,
  tracks NUMBER,
  country TEXT,
  label TEXT,
  catalog_no TEXT,
  status TEXT,
  release_type TEXT,
  date DATE,
  original_date DATE,
  script TEXT,
  UNIQUE(mbid)
);

CREATE TABLE IF NOT EXISTS release_artists (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS tracks (
  mbid BLOB PRIMARY KEY,
  title TEXT NOT NULL,
  length INTEGER,
  disc INTEGER,
  disc_mbid BLOB,
  number INTEGER,
  genres TEXT,
  release BLOB,
  format TEXT,
  path TEXT,
  FOREIGN KEY(release) REFERENCES releases(mbid)
);

CREATE TABLE IF NOT EXISTS track_artists (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_performers (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_engigneers (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_mixers (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_producers (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_lyricists (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_writers (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);

CREATE TABLE IF NOT EXISTS track_composers (
  ref BLOB,
  artist_credit BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist_credit) REFERENCES artist_credits(id),
  UNIQUE(ref,artist_credit)
);
";

#[derive(Debug, FromRow)]
pub struct Release {
    pub mbid: String,
}

#[derive(Debug, FromRow)]
pub struct Track {
    pub mbid: String,
    pub path: String,
}

pub async fn artist_credit(mut ctx: MigrationContext<'_, Sqlite>) -> Result<(), MigrationError> {
    let releases: Vec<Release> = query_as("SELECT mbid FROM releases")
        .fetch_all(ctx.tx())
        .await?;
    if releases.len() == 0 {
        return Ok(());
    }
    let tracks: Vec<Track> = query_as("SELECT mbid, path FROM tracks")
        .fetch_all(ctx.tx())
        .await?;
    let track_paths = tracks.into_iter().fold(HashMap::new(), |mut map, track| {
        map.insert(track.mbid, track.path);
        map
    });
    info!(
        "Migrating {} release{}, {} track{}",
        releases.len(),
        if releases.len() > 0 { 's' } else { '\0' },
        track_paths.len(),
        if track_paths.len() > 0 { 's' } else { '\0' }
    );
    let paths = &track_paths;
    ctx.tx().execute(MUTATE).await?;
    let progress = Arc::new(Mutex::new(ProgressBar::new(releases.len() as u64)));
    let bar = progress.as_ref();

    for rel in releases {
        let (_, mut tracks) = get(&rel.mbid).await.map_err(Error::msg)?;
        for track in tracks.iter_mut() {
            let id = track.mbid.as_ref().unwrap();
            track.path = match paths.get(id) {
                Some(p) => PathBuf::from_str(p).ok(),
                None => {
                    warn!(
                        "Track \"{}\" has changed mbid, the filepath will be reset",
                        id
                    );
                    track.path().ok()
                }
            };
            println!("storing track {:?}", track.mbid);
            track.store(ctx.tx()).await.map_err(Error::msg)?;
        }

        bar.lock().unwrap().inc(1);
    }
    bar.lock().unwrap().finish();
    Ok(())
}
