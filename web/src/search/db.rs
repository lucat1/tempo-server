use async_once_cell::OnceCell;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use std::sync::Arc;
use tantivy::{directory::MmapDirectory, schema::Schema, Index, IndexWriter};
use tokio::sync::Mutex;

use super::schema::{ARTISTS_SCHEMA, RELEASES_SCHEMA, TRACKS_SCHEMA};
use base::{
    setting::{get_settings, Settings},
    util::mkdirp,
};

pub struct Indexes {
    pub artists: Index,
    pub releases: Index,
    pub tracks: Index,
}

pub struct IndexWriters {
    pub artists: IndexWriter,
    pub releases: IndexWriter,
    pub tracks: IndexWriter,
}

lazy_static! {
    pub static ref INDEXES: Arc<OnceCell<Indexes>> = Arc::new(OnceCell::new());
    pub static ref INDEX_WRITERS: Arc<Mutex<OnceCell<IndexWriters>>> =
        Arc::new(Mutex::new(OnceCell::new()));
}

pub fn open_indexes() -> Result<Indexes> {
    let settings = get_settings()?;
    Ok(Indexes {
        artists: open_index(settings, "artists", ARTISTS_SCHEMA.to_owned())?,
        releases: open_index(settings, "releases", RELEASES_SCHEMA.to_owned())?,
        tracks: open_index(settings, "tracks", TRACKS_SCHEMA.to_owned())?,
    })
}

fn open_index(settings: &Settings, resource: &str, schema: Schema) -> Result<Index> {
    let path = settings.search_index.join(resource);
    mkdirp(&path)?;
    Ok(Index::open_or_create(MmapDirectory::open(&path)?, schema)?)
}

pub fn open_index_writers() -> Result<IndexWriters> {
    let Indexes {
        artists,
        releases,
        tracks,
    } = get_indexes()?;
    // 10M should be a plentiful heap size for each collection
    Ok(IndexWriters {
        artists: artists.writer(10 * 1024 * 1024)?,
        releases: releases.writer(10 * 1024 * 1024)?,
        tracks: tracks.writer(10 * 1024 * 1024)?,
    })
}

pub fn get_indexes() -> Result<&'static Indexes> {
    INDEXES.get().ok_or(eyre!("Could not get search indexes"))
}
