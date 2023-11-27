use axum::extract::{OriginalUri, State};
use sea_orm::{
    ColumnTrait, ConnectionTrait, CursorTrait, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::collections::HashMap;

use crate::api::{
    documents::{
        GenreAttributes, GenreFilter, GenreInclude, GenreMetaAttributes, GenreRelation,
        GenreResource, Included, IntoColumn, Meta, ResourceType,
    },
    extract::{Json, Path},
    jsonapi::{
        links_from_resource, make_cursor, Document, DocumentData, Query, Related, Relation,
        Relationship, ResourceIdentifier,
    },
    tempo::{releases, tracks},
    AppState, Error,
};
use base::util::dedup;

#[derive(Default)]
pub struct GenreRelated {
    tracks: Vec<entity::GenreTrack>,
    releases: Vec<entity::GenreRelease>,
}

pub async fn related<C>(
    db: &C,
    entities: &[entity::Genre],
    _light: bool,
) -> Result<Vec<GenreRelated>, Error>
where
    C: ConnectionTrait,
{
    // TODO: limit number of returned relations. Limit even more when light = true
    let tracks = entities.load_many(entity::GenreTrackEntity, db).await?;
    let releases = entities.load_many(entity::GenreReleaseEntity, db).await?;
    let mut result = Vec::with_capacity(entities.len());
    for i in 0..entities.len() {
        let tracks = &tracks[i];
        let releases = &releases[i];
        result.push(GenreRelated {
            tracks: tracks.to_vec(),
            releases: releases.to_vec(),
        })
    }
    Ok(result)
}

pub fn entity_to_resource(entity: &entity::Genre, related: &GenreRelated) -> GenreResource {
    let GenreRelated { tracks, releases } = related;
    let mut relationships = HashMap::new();
    if !tracks.is_empty() {
        relationships.insert(
            GenreRelation::Tracks,
            Relationship {
                data: Relation::Multi(
                    tracks
                        .iter()
                        .map(|t| {
                            Related::Uuid(ResourceIdentifier {
                                r#type: ResourceType::Track,
                                id: t.track_id,
                                meta: Some(Meta::Genre(GenreMetaAttributes { count: t.cnt })),
                            })
                        })
                        .collect(),
                ),
            },
        );
    }
    if !releases.is_empty() {
        relationships.insert(
            GenreRelation::Releases,
            Relationship {
                data: Relation::Multi(
                    releases
                        .iter()
                        .map(|r| {
                            Related::Uuid(ResourceIdentifier {
                                r#type: ResourceType::Release,
                                id: r.release_id,
                                meta: None,
                            })
                        })
                        .collect(),
                ),
            },
        );
    }

    GenreResource {
        r#type: ResourceType::Genre,
        id: entity.id.to_owned(),
        attributes: GenreAttributes {
            name: entity.name.to_owned(),
            disambiguation: entity.disambiguation.to_owned(),
        },
        meta: None,
        relationships,
    }
}

pub async fn included<C>(
    db: &C,
    related: Vec<GenreRelated>,
    include: &[GenreInclude],
) -> Result<Vec<Included>, Error>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&GenreInclude::Tracks) {
        let genre_tracks = related
            .iter()
            .flat_map(|r| r.tracks.to_owned())
            .collect::<Vec<_>>();
        let tracks = genre_tracks
            .load_one(entity::TrackEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let tracks_related = tracks::related(db, &tracks, true).await?;
        for (i, track) in tracks.into_iter().enumerate() {
            included.push(tracks::entity_to_included(&track, &tracks_related[i]));
        }
    }
    if include.contains(&GenreInclude::Releases) {
        let genre_releases = related
            .into_iter()
            .flat_map(|r| r.releases)
            .collect::<Vec<_>>();
        let releases = genre_releases
            .load_one(entity::ReleaseEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let releases_related = releases::related(db, &releases, true).await?;
        for (i, release) in releases.into_iter().enumerate() {
            included.push(releases::entity_to_included(&release, &releases_related[i]));
        }
    }
    Ok(included)
}

pub fn entity_to_included(entity: &entity::Genre, related: &GenreRelated) -> Included {
    Included::Genre(entity_to_resource(entity, related))
}

pub async fn genre(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<GenreFilter, entity::GenreColumn, GenreInclude, String>,
    Path(id): Path<String>,
) -> Result<Json<Document<GenreResource, Included>>, Error> {
    let tx = db.begin().await?;

    let genre = entity::GenreEntity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or(Error::NotFound(None))?;
    let related_to_genres = related(&tx, &[genre.clone()], false).await?;
    let empty_relationship = GenreRelated::default();
    let related = related_to_genres.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&genre, related);
    let included = included(&tx, related_to_genres, &opts.include).await?;
    Ok(Json(Document {
        data: DocumentData::Single(data),
        included: dedup(included),
        links: HashMap::new(),
    }))
}

pub async fn genres(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<GenreFilter, entity::GenreColumn, GenreInclude, String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<GenreResource, Included>>, Error> {
    let tx = db.begin().await?;

    let mut genres_query = entity::GenreEntity::find();
    for (filter_key, filter_value) in opts.filter.iter() {
        if let Some(k) = filter_key.column() {
            genres_query = genres_query.filter(ColumnTrait::eq(&k, filter_value.to_owned()));
        }
    }
    for (sort_key, sort_order) in opts.sort.iter() {
        genres_query = genres_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    let mut _genres_cursor = genres_query.cursor_by(entity::GenreColumn::Id);
    let genres_cursor = make_cursor(&mut _genres_cursor, &opts.page);
    let genres = genres_cursor.all(&tx).await?;
    let related_to_genres = related(&tx, &genres, false).await?;
    let mut data = Vec::new();
    for (i, track) in genres.iter().enumerate() {
        data.push(entity_to_resource(track, &related_to_genres[i]));
    }
    let included = included(&tx, related_to_genres, &opts.include).await?;
    Ok(Json(Document {
        links: links_from_resource(&data, opts, &uri),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}
