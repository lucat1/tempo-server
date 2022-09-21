use crate::models::{Artist, Release, Track};
use crate::{DB, SETTINGS};
use async_trait::async_trait;
use eyre::{eyre, Result, WrapErr};
use serde_json;
use sqlx::{Pool, Row, Sqlite};
use std::path::PathBuf;
use std::time::Duration;

pub trait LibraryRelease {
    fn paths(&self) -> Result<Vec<PathBuf>>;
    fn path(&self) -> Result<PathBuf>;
    fn other_paths(&self) -> Result<Vec<PathBuf>>;
}

impl LibraryRelease for Release {
    fn paths(&self) -> Result<Vec<PathBuf>> {
        let mut v = vec![];
        let settings = SETTINGS
            .get()
            .ok_or(eyre!("Could not read settings"))
            .wrap_err("While generating a path for the library")?;
        for artist in self.artists.iter() {
            let path_str = settings
                .release_name
                .replace("{release.artist}", artist.name.as_str())
                .replace("{release.title}", self.title.as_str());
            v.push(settings.library.join(PathBuf::from(path_str)))
        }
        Ok(v)
    }

    fn path(&self) -> Result<PathBuf> {
        self.paths()?
            .first()
            .map_or(
                Err(eyre!("Release does not have a path in the library, most definitely because the release has no artists")),
                |p| Ok(p.clone())
            )
    }

    fn other_paths(&self) -> Result<Vec<PathBuf>> {
        let main = self.path()?;
        Ok(self
            .paths()?
            .iter()
            .filter_map(|p| -> Option<PathBuf> {
                if *p != main {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }
}

pub trait LibraryTrack {
    fn path(&self, ext: &str) -> Result<PathBuf>;
}

impl LibraryTrack for Track {
    fn path(&self, ext: &str) -> Result<PathBuf> {
        let base = self
            .release
            .clone()
            .ok_or(eyre!("This track doesn't belong to any release"))?
            .path()?;
        let settings = SETTINGS
            .get()
            .ok_or(eyre!("Could not read settings"))
            .wrap_err("While generating a path for the library")?;
        let mut extensionless = settings.track_name.clone();
        extensionless.push('.');
        extensionless.push_str(ext);
        let path_str = extensionless
            .replace(
                "{track.disc}",
                self.disc
                    .ok_or(eyre!("The track has no disc"))?
                    .to_string()
                    .as_str(),
            )
            .replace(
                "{track.number}",
                self.number
                    .ok_or(eyre!("The track has no number"))?
                    .to_string()
                    .as_str(),
            )
            .replace("{track.title}", self.title.as_str());
        Ok(base.join(path_str))
    }
}

#[async_trait]
pub trait Store {
    async fn store(&self) -> Result<()>;
}

#[async_trait]
pub trait Fetch {
    async fn fetch(mbid: String) -> Result<Self>
    where
        Self: Sized;
}

#[async_trait]
impl Fetch for Artist {
    async fn fetch(mbid: String) -> Result<Self> {
        sqlx::query(
            "SELECT (mbid, name, sort_name, instruments) FROM artists WHERE mbid = ? LIMIT 1",
        )
        .bind(mbid)
        .map(|row| {
            Ok(Artist {
                mbid: row.try_get("mbid").ok(),
                name: row.try_get("mbid")?,
                join_phrase: None,
                sort_name: row.try_get("sort_name").ok(),
                instruments: serde_json::from_str(row.try_get("instruments")?)?,
            })
        })
        .fetch_one(DB.get().ok_or(eyre!("Could not get database"))?)
        .await?
    }
}

#[async_trait]
impl Store for Artist {
    async fn store(&self) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO artists (mbid, name, sort_name, instruments) VALUES (?, ?, ?, ?)")
            .bind(&self.mbid)
            .bind(&self.name)
            .bind(&self.sort_name)
            .bind(serde_json::to_string(&self.instruments)?)
            .execute(DB.get().ok_or(eyre!("Could not get database"))?)
            .await?;
        Ok(())
    }
}

async fn get_link(
    db: &Pool<Sqlite>,
    table: &str,
    key: &str,
    mbid: Option<&String>,
) -> Result<Vec<Artist>> {
    sqlx::query(
        format!(
            "SELECT ({}, artist) FROM {} WHERE {} = ? LIMIT 1",
            key, table, key
        )
        .as_str(),
    )
    .bind(mbid)
    .map(|row| {
        Ok(Artist {
            mbid: row.try_get("mbid").ok(),
            name: row.try_get("mbid")?,
            join_phrase: None,
            sort_name: row.try_get("sort_name").ok(),
            instruments: serde_json::from_str(row.try_get("instruments")?)?,
        })
    })
    .fetch_all(db)
    .await?
    .into_iter()
    .collect()
}

async fn link(
    db: &Pool<Sqlite>,
    table: &str,
    key: &str,
    mbid: Option<&String>,
    artist: &Artist,
) -> Result<()> {
    sqlx::query(
        format!(
            "INSERT OR REPLACE INTO {} ({}, artist) VALUES (?, ?)",
            table, key
        )
        .as_str(),
    )
    .bind(mbid)
    .bind(artist.mbid.as_ref())
    .execute(db)
    .await?;
    Ok(())
}

#[async_trait]
impl Fetch for Track {
    async fn fetch(mbid: String) -> Result<Self> {
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        let mut track = sqlx::query(
            "SELECT (mbid, title, length, disc, disc_mbid, number, genres, release) FROM tracks WHERE mbid = ? LIMIT 1",
        )
        .bind(mbid)
        .map(|row| -> Result<Track> {
            Ok(Track {
                mbid: row.try_get("mbid").ok(),
                title: row.try_get("title")?,
                artists: vec![],
                length: row.try_get("title").ok().map(|d: i64| Duration::from_secs(d as u64)),
                disc: row.try_get("disc").ok().map(|d: i64| d as u64),
                disc_mbid: row.try_get("disc_mbid").ok(),
                number: row.try_get("number").ok().map(|d: i64| d as u64),
                genres: serde_json::from_str(row.try_get("genres")?)?,
                release: None,

                performers: vec![],
                engigneers: vec![],
                mixers: vec![],
                producers: vec![],
                lyricists: vec![],
                writers: vec![],
                composers: vec![],
            })
        })
        .fetch_one(db)
        .await??;
        track.artists = get_link(db, "track_artists", "track", track.mbid.as_ref()).await?;
        track.performers = get_link(db, "track_performers", "track", track.mbid.as_ref()).await?;
        track.engigneers = get_link(db, "track_engigneers", "track", track.mbid.as_ref()).await?;
        track.mixers = get_link(db, "track_mixers", "track", track.mbid.as_ref()).await?;
        track.producers = get_link(db, "track_producers", "track", track.mbid.as_ref()).await?;
        track.lyricists = get_link(db, "track_lyricists", "track", track.mbid.as_ref()).await?;
        track.writers = get_link(db, "track_writers", "track", track.mbid.as_ref()).await?;
        track.composers = get_link(db, "track_composers", "track", track.mbid.as_ref()).await?;
        Ok(track)
    }
}

#[async_trait]
impl Store for Track {
    async fn store(&self) -> Result<()> {
        if let Some(rel) = &self.release {
            rel.store().await?;
        }
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        sqlx::query("INSERT OR REPLACE INTO tracks (mbid, title, length, disc, disc_mbid, number, genres, release) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&self.mbid)
            .bind(&self.title)
            .bind(&self.length.map(|t| t.as_secs() as i64))
            .bind(&self.disc.map(|n| n as i64))
            .bind(&self.disc_mbid)
            .bind(&self.number.map(|n| n as i64))
            .bind(serde_json::to_string(&self.genres)?)
            .bind(self.release.as_ref().map_or(None, |r| r.mbid.as_ref()))
            .execute(db)
            .await?;

        for artist in self.artists.iter() {
            artist.store().await?;
            link(db, "track_artists", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.performers.iter() {
            artist.store().await?;
            link(db, "track_performers", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.engigneers.iter() {
            artist.store().await?;
            link(db, "track_engigneers", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.mixers.iter() {
            artist.store().await?;
            link(db, "track_mixers", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.producers.iter() {
            artist.store().await?;
            link(db, "track_producers", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.lyricists.iter() {
            artist.store().await?;
            link(db, "track_lyricists", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.writers.iter() {
            artist.store().await?;
            link(db, "track_writers", "track", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.composers.iter() {
            artist.store().await?;
            link(db, "track_composers", "track", self.mbid.as_ref(), artist).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Store for Release {
    async fn store(&self) -> Result<()> {
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        sqlx::query("INSERT OR REPLACE INTO releases (mbid, release_group_mbid, asin, title, discs, media, tracks, country, label, catalog_no, status, release_type, date, original_date, script) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&self.mbid)
            .bind(&self.release_group_mbid)
            .bind(&self.asin)
            .bind(&self.title)
            .bind(&self.discs.map(|n| n as i64))
            .bind(&self.tracks.map(|n| n as i64))
            .bind(&self.country)
            .bind(&self.label)
            .bind(&self.catalog_no)
            .bind(&self.status)
            .bind(&self.release_type)
            .bind(&self.date)
            .bind(&self.original_date)
            .bind(&self.script)
            .execute(db)
            .await?;
        for artist in self.artists.iter() {
            artist.store().await?;
            link(db, "release_artists", "release", self.mbid.as_ref(), artist).await?;
        }
        Ok(())
    }
}
