use axum::extract::{Query, State};
use axum::http::StatusCode;
use eyre::{eyre, Result};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::collections::HashMap;
use tantivy::{collector::TopDocs, query::QueryParser, schema::Value, ReloadPolicy};
use uuid::Uuid;

use super::{artists, releases, tracks};
use crate::api::{
    documents::{
        dedup, ArtistResource, Included, Meta, ReleaseResource, SearchResultAttributes,
        TrackResource,
    },
    extract::Json,
    jsonapi::{Document, DocumentData, Error},
    AppState,
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

#[derive(Deserialize, Validate)]
pub struct SearchQuery {
    query: String,
    #[serde(default = "default_limit")]
    #[validate(maximum = 50)]
    limit: u32,
}

fn default_limit() -> u32 {
    25
}

#[derive(Clone, Copy, Debug)]
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
    let ids = get_ids(artists_docs)?;
    let mut data = Vec::new();

    match index {
        Index::Artists(_) => {
            let mut cond = Condition::any();
            for (_score, id) in ids.iter() {
                cond = cond.add(ColumnTrait::eq(&entity::ArtistColumn::Id, id.to_owned()));
            }
            let artists = entity::ArtistEntity::find().filter(cond).all(db).await?;

            let related_to_artists = artists::related(db, &artists, false).await?;
            for (i, artist) in artists.iter().enumerate() {
                let mut entity = artists::entity_to_resource(artist, &related_to_artists[i]);
                entity.meta = Some(Meta::SearchResult(SearchResultAttributes {
                    score: ids[i].0,
                }));
                data.push(SearchResult::Artist(entity));
            }
        }
        Index::Releases(_) => {
            let mut cond = Condition::any();
            for (_score, id) in ids.iter() {
                cond = cond.add(ColumnTrait::eq(&entity::ReleaseColumn::Id, id.to_owned()));
            }
            let releases = entity::ReleaseEntity::find().filter(cond).all(db).await?;

            let related_to_releases = releases::related(db, &releases, false).await?;
            for (i, release) in releases.iter().enumerate() {
                let mut entity = releases::entity_to_resource(release, &related_to_releases[i]);
                entity.meta = Some(Meta::SearchResult(SearchResultAttributes {
                    score: ids[i].0,
                }));
                data.push(SearchResult::Release(entity));
            }
        }
        Index::Tracks(_) => {
            let mut cond = Condition::any();
            for (_score, id) in ids.iter() {
                cond = cond.add(ColumnTrait::eq(&entity::TrackColumn::Id, id.to_owned()));
            }
            let tracks = entity::TrackEntity::find().filter(cond).all(db).await?;

            let related_to_tracks = tracks::related(db, &tracks, false).await?;
            for (i, track) in tracks.iter().enumerate() {
                let mut entity = tracks::entity_to_resource(track, &related_to_tracks[i]);
                entity.meta = Some(Meta::SearchResult(SearchResultAttributes {
                    score: ids[i].0,
                }));
                data.push(SearchResult::Track(entity));
            }
        }
    }

    Ok(data)
}

// TODO: includes?
pub async fn search(
    State(AppState(db)): State<AppState>,
    Query(search): Query<SearchQuery>,
) -> Result<Json<Document<SearchResult, Included>>, Error> {
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
    let mut artists = search_and_map(&tx, Index::Artists(&indexes.artists), &search)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not search for artists".to_string(),
            detail: Some(e.into()),
        })?;

    let mut releases = search_and_map(&tx, Index::Releases(&indexes.releases), &search)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not search for releases".to_string(),
            detail: Some(e.into()),
        })?;
    tracing::info!(len = ?releases.len(), "Releases");

    let mut tracks = search_and_map(&tx, Index::Tracks(&indexes.tracks), &search)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not search for tracks".to_string(),
            detail: Some(e.into()),
        })?;

    let mut results = Vec::new();
    results.append(&mut artists);
    results.append(&mut releases);
    results.append(&mut tracks);

    Ok(Json::new(Document {
        links: HashMap::new(),
        data: DocumentData::Multi(results),
        included: dedup(Vec::new()),
    }))
}
