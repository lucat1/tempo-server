use async_once_cell::OnceCell;
use lazy_static::lazy_static;
use std::sync::Arc;
use tantivy::query::QueryParserError;
use tantivy::{
    collector::TopDocs, directory::error::OpenDirectoryError, directory::MmapDirectory,
    query::QueryParser, schema::Schema, schema::Value, IndexWriter, ReloadPolicy, TantivyError,
};
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::documents::{artist_fields, release_fields, track_fields};
use super::schema::{ARTISTS_SCHEMA, RELEASES_SCHEMA, TRACKS_SCHEMA};
use base::{
    setting::{get_settings, Settings, SettingsError},
    util::mkdirp,
};

pub struct Indexes {
    pub artists: tantivy::Index,
    pub releases: tantivy::Index,
    pub tracks: tantivy::Index,
}

pub struct IndexWriters {
    pub artists: IndexWriter,
    pub releases: IndexWriter,
    pub tracks: IndexWriter,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Indexes store uninitialized")]
    Uninitialized,

    #[error("Could not get settings: {0}")]
    Settings(#[from] SettingsError),

    #[error("Error in the indexer library: {0}")]
    Tantivy(#[from] TantivyError),

    #[error("Could not open index store: {0}")]
    IO(#[from] std::io::Error),

    #[error("Could not open store directory: {0}")]
    DirectoryError(#[from] OpenDirectoryError),

    #[error("Could not load artist fields")]
    NoArtistFields,
    #[error("Could not load track fields")]
    NoTrackFields,
    #[error("Could not load release fields")]
    NoReleaseFields,

    #[error("Unexpected search result id type")]
    InvalidResultType,
    #[error("Result not found")]
    FieldNotFound,
    #[error("Could not parse field as uuid: {0}")]
    UuidParseError(#[from] uuid::Error),

    #[error("Invalid query: {0}")]
    InvalidQuery(#[from] QueryParserError),
}

lazy_static! {
    pub static ref INDEXES: Arc<OnceCell<Indexes>> = Arc::new(OnceCell::new());
    pub static ref INDEX_WRITERS: Arc<Mutex<OnceCell<IndexWriters>>> =
        Arc::new(Mutex::new(OnceCell::new()));
}

pub fn open_indexes() -> Result<Indexes, SearchError> {
    let settings = get_settings()?;
    Ok(Indexes {
        artists: open_index(settings, "artists", ARTISTS_SCHEMA.to_owned())?,
        releases: open_index(settings, "releases", RELEASES_SCHEMA.to_owned())?,
        tracks: open_index(settings, "tracks", TRACKS_SCHEMA.to_owned())?,
    })
}

fn open_index(
    settings: &Settings,
    resource: &str,
    schema: Schema,
) -> Result<tantivy::Index, SearchError> {
    let path = settings.search_index.join(resource);
    mkdirp(&path)?;
    Ok(tantivy::Index::open_or_create(
        MmapDirectory::open(&path)?,
        schema,
    )?)
}

pub fn open_index_writers() -> Result<IndexWriters, SearchError> {
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

pub fn get_indexes() -> Result<&'static Indexes, SearchError> {
    INDEXES.get().ok_or(SearchError::Uninitialized)
}

#[derive(Clone, Copy, Debug)]
pub enum Index<'a> {
    Artists(&'a tantivy::Index),
    Releases(&'a tantivy::Index),
    Tracks(&'a tantivy::Index),
}

pub fn do_search(index: Index, query: &str, limit: u32) -> Result<Vec<(f32, Value)>, SearchError> {
    let anyway_index = match index {
        Index::Artists(i) => i,
        Index::Releases(i) => i,
        Index::Tracks(i) => i,
    };
    let reader = anyway_index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()?;
    let (id_field, query_parser) = match index {
        Index::Artists(i) => {
            let fields = artist_fields().ok_or(SearchError::NoArtistFields)?;
            (
                fields.id,
                QueryParser::for_index(i, vec![fields.name, fields.sort_name, fields.description]),
            )
        }
        Index::Tracks(i) => {
            let fields = track_fields().ok_or(SearchError::NoTrackFields)?;
            (
                fields.id,
                QueryParser::for_index(i, vec![fields.artists, fields.title, fields.genres]),
            )
        }
        Index::Releases(i) => {
            let fields = release_fields().ok_or(SearchError::NoReleaseFields)?;
            (
                fields.id,
                QueryParser::for_index(
                    i,
                    vec![
                        fields.artists,
                        fields.title,
                        fields.release_type,
                        fields.genres,
                    ],
                ),
            )
        }
    };
    let query = query_parser.parse_query(query)?;
    let searcher = reader.searcher();
    let results = searcher.search(&query, &TopDocs::with_limit(limit as usize))?;
    results
        .into_iter()
        .map(|(score, addr)| -> Result<(f32, Value), SearchError> {
            let doc = searcher.doc(addr)?;
            Ok((
                score,
                doc.get_first(id_field)
                    .ok_or(SearchError::FieldNotFound)?
                    .to_owned(),
            ))
        })
        .collect()
}

pub fn get_ids(results: Vec<(f32, Value)>) -> Result<Vec<(f32, Uuid)>, SearchError> {
    results
        .into_iter()
        .map(|(score, value)| -> Result<(f32, Uuid), SearchError> {
            match value {
                Value::Str(id) => Ok((score, id.parse::<Uuid>()?)),
                _ => Err(SearchError::InvalidResultType),
            }
        })
        .collect()
}
