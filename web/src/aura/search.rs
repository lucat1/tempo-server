use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use eyre::{bail, eyre, Result};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbConn, EntityTrait, QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tantivy::{collector::TopDocs, query::QueryParser, schema::Value, ReloadPolicy};
use uuid::Uuid;

use super::artists;
use crate::jsonapi::{
    dedup, AppState, ArtistResource, Document, DocumentData, Error, ReleaseResource,
    ResourceMetaKey, TrackResource,
};
use crate::search::{
    documents::{artist_fields, release_fields, track_fields},
    get_indexes,
};

#[derive(Serialize)]
#[serde(untagged)]
pub enum SearchResult {
    Artist(ArtistResource),
    Release(ReleaseResource),
    Track(TrackResource),
}

#[derive(Deserialize)]
pub struct SearchQuery {
    query: String,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    20
}

#[derive(Clone, Copy)]
enum Index<'a> {
    Artists(&'a tantivy::Index),
    Releases(&'a tantivy::Index),
    Tracks(&'a tantivy::Index),
}

fn do_search(index: Index, search: &SearchQuery) -> Result<Vec<(f32, Value)>> {
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
            let fields = artist_fields().ok_or(eyre!("Could not get search artist fields"))?;
            (
                fields.id,
                QueryParser::for_index(i, vec![fields.name, fields.sort_name, fields.description]),
            )
        }
        Index::Tracks(i) => {
            let fields = track_fields().ok_or(eyre!("Could not get search track fields"))?;
            (
                fields.id,
                QueryParser::for_index(i, vec![fields.artists, fields.title, fields.genres]),
            )
        }
        Index::Releases(i) => {
            let fields = release_fields().ok_or(eyre!("Could not get search artist fields"))?;
            (
                fields.id,
                QueryParser::for_index(
                    i,
                    vec![
                        fields.artists,
                        fields.title,
                        fields.release_type,
                        fields.genres,
                        fields.date,
                        fields.original_date,
                    ],
                ),
            )
        }
    };
    let query = query_parser.parse_query(search.query.as_str())?;
    let searcher = reader.searcher();
    let results = searcher.search(&query, &TopDocs::with_limit(search.limit as usize))?;
    results
        .into_iter()
        .map(|(score, addr)| -> Result<(f32, Value)> {
            let doc = searcher.doc(addr)?;
            Ok((
                score,
                doc.get_first(id_field)
                    .ok_or(eyre!("document doesn't have an id"))?
                    .to_owned(),
            ))
        })
        .collect()
}

fn get_ids(results: Vec<(f32, Value)>) -> Result<Vec<(f32, Uuid)>> {
    results
        .into_iter()
        .map(|(score, value)| -> Result<(f32, Uuid)> {
            match value {
                Value::Str(id) => Ok((score, id.parse::<Uuid>()?)),
                _ => Err(eyre!("Unexpected search result id type")),
            }
        })
        .collect()
}

async fn search_and_map<'a, C>(
    db: &C,
    index: Index<'a>,
    search: &SearchQuery,
) -> Result<Vec<SearchResult>>
where
    C: ConnectionTrait,
{
    let artists_docs = do_search(index, search)?;
    let artist_ids = get_ids(artists_docs)?;
    let mut cond = Condition::any();
    for (_score, id) in artist_ids.iter() {
        cond = cond.add(ColumnTrait::eq(&entity::ArtistColumn::Id, id.to_owned()));
    }
    let artists = entity::ArtistEntity::find().filter(cond).all(db).await?;

    let related_to_artists = artists::related(db, &artists, false).await?;
    let mut data = Vec::new();
    for (i, artist) in artists.iter().enumerate() {
        let mut entity = artists::entity_to_resource(artist, &related_to_artists[i]);
        entity
            .meta
            .insert(ResourceMetaKey::Score, artist_ids[i].0.to_string());
        data.push(SearchResult::Artist(entity));
    }

    Ok(data)
}

// TODO: includes?
pub async fn search(
    State(AppState(db)): State<AppState>,
    Query(search): Query<SearchQuery>,
) -> Result<Json<Document<SearchResult>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let indexes = get_indexes().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't get a hold of the search indexes".to_string(),
        detail: Some(e.into()),
    })?;
    let artists = search_and_map(&tx, Index::Artists(&indexes.artists), &search)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not search for artists".to_string(),
            detail: Some(e.into()),
        })?;

    Ok(Json(Document {
        links: HashMap::new(),
        data: DocumentData::Multi(artists),
        included: dedup(Vec::new()),
    }))
}
