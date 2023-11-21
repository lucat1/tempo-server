use axum::extract::{Query, State};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::collections::HashMap;

use crate::api::{
    documents::{
        ArtistResource, Included, Meta, ReleaseResource, SearchResultAttributes, TrackResource,
    },
    extract::Json,
    jsonapi::{Document, DocumentData},
    tempo::{artists, error::TempoError, releases, tracks},
    AppState,
};
use crate::search::{
    db::{do_search, get_ids, Index},
    get_indexes,
};
use base::util::dedup;

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

async fn search_and_map<'a, C>(
    db: &C,
    index: Index<'a>,
    search: &SearchQuery,
) -> Result<Vec<SearchResult>, TempoError>
where
    C: ConnectionTrait,
{
    let artists_docs = do_search(index, &search.query, search.limit)?;
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
) -> Result<Json<Document<SearchResult, Included>>, TempoError> {
    let tx = db.begin().await?;
    let indexes = get_indexes()?;
    let mut artists = search_and_map(&tx, Index::Artists(&indexes.artists), &search).await?;
    let mut releases = search_and_map(&tx, Index::Releases(&indexes.releases), &search).await?;
    let mut tracks = search_and_map(&tx, Index::Tracks(&indexes.tracks), &search).await?;

    let mut results = Vec::new();
    results.append(&mut artists);
    results.append(&mut releases);
    results.append(&mut tracks);

    Ok(Json(Document {
        links: HashMap::new(),
        data: DocumentData::Multi(results),
        included: dedup(Vec::new()),
    }))
}
