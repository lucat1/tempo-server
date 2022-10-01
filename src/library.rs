use crate::models::{Artist, Format, Release, Track};
use crate::track::format::Format as TrackFormat;
use crate::util::path_to_str;
use crate::{DB, SETTINGS};
use async_trait::async_trait;
use eyre::{eyre, Result, WrapErr};
use itertools::Itertools;
use log::trace;
use sqlx::sqlite::SqliteRow;
use sqlx::{Encode, Pool, QueryBuilder, Row, Sqlite, Type};
use std::fmt::Display;
use std::iter;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub trait LibraryTrack {
    fn path(&self) -> Result<PathBuf>;
}

impl LibraryTrack for Track {
    fn path(&self) -> Result<PathBuf> {
        let settings = SETTINGS
            .get()
            .ok_or(eyre!("Could not read settings"))
            .wrap_err("While generating a path for the library")?;
        let mut builder = self.fmt(settings.track_name.as_str())?;
        builder.push('.');
        builder.push_str(
            self.format
                .ok_or(eyre!("The given Track doesn't have an associated format"))?
                .ext(),
        );
        Ok(settings
            .library
            .join(PathBuf::from_str(builder.as_str()).map_err(|e| eyre!(e))?))
    }
}

pub trait Value<'args>: Encode<'args, Sqlite> + sqlx::Type<Sqlite> {}

#[async_trait]
pub trait InTable {
    fn table() -> &'static str;
    fn fields() -> Vec<&'static str>;
    fn store_fields() -> Vec<&'static str>;
    fn join() -> Option<&'static str>;
    fn decode(row: SqliteRow) -> Result<Self, sqlx::Error>
    where
        Self: Sized;
    async fn fill_relationships(&mut self, db: &Pool<Sqlite>) -> Result<()>;
}

pub trait Builder: InTable {
    fn query_builder<'args, B, D>(
        fields: Vec<(D, B)>,
        extra: Vec<D>,
    ) -> QueryBuilder<'args, Sqlite>
    where
        B: 'args + Encode<'args, Sqlite> + Send + Type<Sqlite>,
        D: Display;
    fn store_builder<'args>() -> QueryBuilder<'args, Sqlite>;
}

#[async_trait]
pub trait Filter: Builder {
    async fn filter<'args, B, D>(fields: Vec<(D, B)>, extra: Vec<D>) -> Result<Vec<Self>>
    where
        B: 'args + Encode<'args, Sqlite> + Send + Type<Sqlite>,
        D: Display + Send,
        Self: Sized;
}

#[async_trait]
pub trait Fetch: Builder {
    async fn fetch(mbid: String) -> Result<Self>
    where
        Self: Sized;
}

#[async_trait]
pub trait Store: Builder {
    async fn store(&self) -> Result<()>;
}

impl<T> Builder for T
where
    T: InTable,
{
    fn query_builder<'args, B, D>(fields: Vec<(D, B)>, extra: Vec<D>) -> QueryBuilder<'args, Sqlite>
    where
        B: 'args + Encode<'args, Sqlite> + Send + Type<Sqlite>,
        D: Display,
    {
        let mut qb = QueryBuilder::new("SELECT ");
        qb.push(Self::fields().join(","));
        qb.push(" FROM ");
        qb.push(Self::table());
        if !fields.is_empty() {
            qb.push(" WHERE ");
            let len = fields.len();
            for (i, (key, val)) in fields.into_iter().enumerate() {
                qb.push(format!("{} = ", key));
                qb.push_bind(val);
                if i < len - 1 {
                    qb.push(" AND ");
                }
            }
        }
        if let Some(join) = Self::join() {
            qb.push(join);
        }
        for ex in extra.into_iter() {
            qb.push(ex);
        }
        trace!("Building query: {}", qb.sql());
        qb
    }
    fn store_builder<'args>() -> QueryBuilder<'args, Sqlite> {
        let mut qb = QueryBuilder::new("INSERT OR REPLACE INTO ");
        qb.push(Self::table());
        qb.push(" (");
        qb.push(Self::store_fields().join(","));
        qb.push(") VALUES (");
        qb.push(iter::repeat("?").take(Self::store_fields().len()).join(","));
        qb.push(")");
        trace!("Building query: {}", qb.sql());
        qb
    }
}

#[async_trait]
impl<T> Filter for T
where
    T: Builder + Send + Unpin,
{
    async fn filter<'args, B, D>(fields: Vec<(D, B)>, extra: Vec<D>) -> Result<Vec<Self>>
    where
        B: 'args + Encode<'args, Sqlite> + Send + Type<Sqlite>,
        D: Display + Send,
        Self: Sized,
    {
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        let mut vals = Self::query_builder(fields, extra)
            .build()
            .try_map(Self::decode)
            .fetch_all(db)
            .await?;
        for val in vals.iter_mut() {
            val.fill_relationships(db).await?;
        }
        Ok(vals)
    }
}

#[async_trait]
impl<T> Fetch for T
where
    T: Builder + Send + Unpin,
{
    async fn fetch(mbid: String) -> Result<Self>
    where
        Self: Sized,
    {
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        let mut val = Self::query_builder(vec![("mbid", mbid)], vec!["LIMIT 1"])
            .build()
            .try_map(Self::decode)
            .fetch_one(db)
            .await?;
        val.fill_relationships(db).await?;
        Ok(val)
    }
}

#[async_trait]
impl InTable for Artist {
    fn table() -> &'static str {
        "artists"
    }
    fn fields() -> Vec<&'static str> {
        vec!["mbid", "name", "sort_name", "instruments"]
    }
    fn store_fields() -> Vec<&'static str> {
        Artist::fields()
    }
    fn join() -> Option<&'static str> {
        None
    }
    fn decode(row: SqliteRow) -> Result<Self, sqlx::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            mbid: row.try_get("mbid").ok(),
            name: row.try_get("name")?,
            join_phrase: None,
            sort_name: row.try_get("sort_name").ok(),
            instruments: serde_json::from_str(row.try_get("instruments")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        })
    }
    async fn fill_relationships(&mut self, _: &Pool<Sqlite>) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl Store for Artist {
    async fn store(&self) -> Result<()> {
        Self::store_builder()
            .build()
            .bind(&self.mbid)
            .bind(&self.name)
            .bind(&self.sort_name)
            .bind(serde_json::to_string(&self.instruments)?)
            .execute(DB.get().ok_or(eyre!("Could not get database"))?)
            .await?;
        Ok(())
    }
}

async fn resolve(db: &Pool<Sqlite>, table: &str, mbid: Option<&String>) -> Result<Vec<Artist>> {
    Artist::query_builder::<String, String>(vec![], vec![])
        .push(format!(
            " WHERE mbid = (SELECT artist FROM {} WHERE ref =",
            table
        ))
        .push_bind(mbid)
        .push(")")
        .build()
        .try_map(Artist::decode)
        .fetch_all(db)
        .await
        .map_err(|e| eyre!(e))
}

async fn link(
    db: &Pool<Sqlite>,
    table: &str,
    mbid: Option<&String>,
    artist: &Artist,
) -> Result<()> {
    sqlx::query(
        format!(
            "INSERT OR REPLACE INTO {} (ref, artist) VALUES (?, ?)",
            table
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
impl InTable for Track {
    fn table() -> &'static str {
        "tracks"
    }
    fn fields() -> Vec<&'static str> {
        vec![
            "tracks.mbid AS t_mbid",
            "tracks.title AS t_title",
            "tracks.length AS t_length",
            "tracks.disc AS t_disc",
            "tracks.disc_mbid AS t_disc_mbid",
            "tracks.number AS t_number",
            "tracks.genres AS t_genres",
            // "tracks.release AS t_release",
            "tracks.format AS t_format",
            "tracks.path AS t_path",
            "releases.mbid AS r_mbid",
            "releases.release_group_mbid AS r_release_group_mbid",
            "releases.asin AS r_asin",
            "releases.title AS r_title",
            "releases.discs AS r_discs",
            "releases.media AS r_media",
            "releases.tracks AS r_tracks",
            "releases.country AS r_country",
            "releases.label AS r_label",
            "releases.catalog_no AS r_catalog_no",
            "releases.status AS r_status",
            "releases.release_type AS r_release_type",
            "releases.date AS r_date",
            "releases.original_date AS r_original_date",
            "releases.script AS r_script",
        ]
    }
    fn store_fields() -> Vec<&'static str> {
        vec![
            "mbid",
            "title",
            "length",
            "disc",
            "disc_mbid",
            "number",
            "genres",
            "release",
            "format",
            "path",
        ]
    }
    fn join() -> Option<&'static str> {
        Some(" INNER JOIN releases ON releases.mbid = tracks.release")
    }
    fn decode(row: SqliteRow) -> Result<Self, sqlx::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            mbid: row.try_get("t_mbid").ok(),
            title: row.try_get("t_title")?,
            artists: vec![],
            length: row
                .try_get("t_title")
                .ok()
                .map(|d: i64| Duration::from_secs(d as u64)),
            disc: row.try_get("t_disc").ok().map(|d: i64| d as u64),
            disc_mbid: row.try_get("t_disc_mbid").ok(),
            number: row.try_get("t_number").ok().map(|d: i64| d as u64),
            genres: serde_json::from_str(row.try_get("t_genres")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,

            performers: vec![],
            engigneers: vec![],
            mixers: vec![],
            producers: vec![],
            lyricists: vec![],
            writers: vec![],
            composers: vec![],

            format: row
                .try_get("t_format")
                .map_or(Ok(None), |f| TrackFormat::from_ext(f).map(Some))
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            path: row
                .try_get("t_path")
                .map_or(None, |p: &str| PathBuf::from_str(p).ok()),

            release: Some(Release::decode(row)?),
        })
    }
    async fn fill_relationships(&mut self, db: &Pool<Sqlite>) -> Result<()> {
        if let Some(mut release) = self.release.as_mut() {
            release.artists = resolve(db, "release_artists", release.mbid.as_ref()).await?;
        }
        self.artists = resolve(db, "track_artists", self.mbid.as_ref()).await?;
        self.performers = resolve(db, "track_performers", self.mbid.as_ref()).await?;
        self.engigneers = resolve(db, "track_engigneers", self.mbid.as_ref()).await?;
        self.mixers = resolve(db, "track_mixers", self.mbid.as_ref()).await?;
        self.producers = resolve(db, "track_producers", self.mbid.as_ref()).await?;
        self.lyricists = resolve(db, "track_lyricists", self.mbid.as_ref()).await?;
        self.writers = resolve(db, "track_writers", self.mbid.as_ref()).await?;
        self.composers = resolve(db, "track_composers", self.mbid.as_ref()).await?;
        Ok(())
    }
}

#[async_trait]
impl Store for Track {
    async fn store(&self) -> Result<()> {
        if let Some(rel) = &self.release {
            rel.store().await?;
        }
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        Track::store_builder()
            .build()
            .bind(&self.mbid)
            .bind(&self.title)
            .bind(&self.length.map(|t| t.as_secs() as i64))
            .bind(&self.disc.map(|n| n as i64))
            .bind(&self.disc_mbid)
            .bind(&self.number.map(|n| n as i64))
            .bind(serde_json::to_string(&self.genres)?)
            .bind(self.release.as_ref().and_then(|r| r.mbid.as_ref()))
            .bind(self.format.map(String::from))
            .bind(self.path.as_ref().map_or(
                Err(eyre!("The given track doesn't have an associated path")),
                path_to_str,
            )?)
            .execute(db)
            .await?;

        for artist in self.artists.iter() {
            artist.store().await?;
            link(db, "track_artists", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.performers.iter() {
            artist.store().await?;
            link(db, "track_performers", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.engigneers.iter() {
            artist.store().await?;
            link(db, "track_engigneers", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.mixers.iter() {
            artist.store().await?;
            link(db, "track_mixers", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.producers.iter() {
            artist.store().await?;
            link(db, "track_producers", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.lyricists.iter() {
            artist.store().await?;
            link(db, "track_lyricists", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.writers.iter() {
            artist.store().await?;
            link(db, "track_writers", self.mbid.as_ref(), artist).await?;
        }
        for artist in self.composers.iter() {
            artist.store().await?;
            link(db, "track_composers", self.mbid.as_ref(), artist).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl InTable for Release {
    fn table() -> &'static str {
        "releases"
    }
    fn fields() -> Vec<&'static str> {
        vec![
            "mbid AS r_mbid",
            "release_group_mbid AS r_release_group_mbid",
            "asin AS r_asin",
            "title AS r_title",
            "discs AS r_discs",
            "media AS r_media",
            "tracks AS r_tracks",
            "country AS r_country",
            "label AS r_label",
            "catalog_no AS r_catalog_no",
            "status AS r_status",
            "release_type AS r_release_type",
            "date AS r_date",
            "original_date AS r_original_date",
            "script AS r_script",
        ]
    }
    fn store_fields() -> Vec<&'static str> {
        vec![
            "mbid",
            "release_group_mbid",
            "asin",
            "title",
            "discs",
            "media",
            "tracks",
            "country",
            "label",
            "catalog_no",
            "status",
            "release_type",
            "date",
            "original_date",
            "script",
        ]
    }
    fn join() -> Option<&'static str> {
        None
    }
    fn decode(row: SqliteRow) -> Result<Self, sqlx::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            mbid: row.try_get("r_mbid").ok(),
            release_group_mbid: row.try_get("r_release_group_mbid").ok(),
            asin: row.try_get("r_asin").ok(),
            title: row.try_get("r_title")?,
            artists: vec![],
            discs: row.try_get("r_discs").ok().map(|d: i64| d as u64),
            media: row.try_get("r_media").ok(),
            tracks: row.try_get("r_tracks").ok().map(|d: i64| d as u64),
            country: row.try_get("r_country").ok(),
            label: row.try_get("r_label").ok(),
            catalog_no: row.try_get("r_catalog_no").ok(),
            status: row.try_get("r_status").ok(),
            release_type: row.try_get("r_release_type").ok(),
            date: row.try_get("r_date").ok(),
            original_date: row.try_get("r_original_date").ok(),
            script: row.try_get("r_script").ok(),
        })
    }
    async fn fill_relationships(&mut self, db: &Pool<Sqlite>) -> Result<()> {
        self.artists = resolve(db, "release_artists", self.mbid.as_ref()).await?;
        Ok(())
    }
}

#[async_trait]
impl Store for Release {
    async fn store(&self) -> Result<()> {
        let db = DB.get().ok_or(eyre!("Could not get database"))?;
        Release::store_builder()
            .build()
            .bind(&self.mbid)
            .bind(&self.release_group_mbid)
            .bind(&self.asin)
            .bind(&self.title)
            .bind(&self.discs.map(|n| n as i64))
            .bind(&self.media)
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
            link(db, "release_artists", self.mbid.as_ref(), artist).await?;
        }
        Ok(())
    }
}
