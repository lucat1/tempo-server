use async_recursion::async_recursion;
use axum::extract::{OriginalUri, State};
use sea_orm::{
    ColumnTrait, ConnectionTrait, CursorTrait, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::{
    documents::ReleaseInclude,
    documents::{
        Included, IntoColumn, MediumAttributes, MediumFilter, MediumInclude, MediumRelation,
        MediumResource, ResourceType, TrackInclude,
    },
    extract::{Json, Path},
    jsonapi::{
        links_from_resource, make_cursor, Document, DocumentData, Query, Related, Relation,
        Relationship, ResourceIdentifier,
    },
    tempo::{error::TempoError, releases, tracks},
    AppState,
};
use base::util::dedup;

#[derive(Default)]
pub struct MediumRelated {
    pub release: Option<entity::Release>,
    pub tracks: Vec<entity::Track>,
}

pub async fn related<C>(
    db: &C,
    entities: &[entity::Medium],
    _light: bool,
) -> Result<Vec<MediumRelated>, TempoError>
where
    C: ConnectionTrait,
{
    let releases_tracks = entities.load_many(entity::TrackEntity, db).await?;
    let releases = entities.load_one(entity::ReleaseEntity, db).await?;

    let mut related = Vec::new();
    for i in 0..entities.len() {
        let tracks = &releases_tracks[i];
        let release = &releases[i];

        related.push(MediumRelated {
            release: release.to_owned(),
            tracks: tracks.to_owned(),
        });
    }

    Ok(related)
}

pub fn entity_to_resource(entity: &entity::Medium, related: &MediumRelated) -> MediumResource {
    let MediumRelated { release, tracks } = related;
    let mut relationships = HashMap::new();
    if let Some(rel) = release {
        relationships.insert(
            MediumRelation::Release,
            Relationship {
                data: Relation::Single(Related::Uuid(ResourceIdentifier {
                    r#type: ResourceType::Release,
                    id: rel.id,
                    meta: None,
                })),
            },
        );
    }
    if !tracks.is_empty() {
        relationships.insert(
            MediumRelation::Tracks,
            Relationship {
                data: Relation::Multi(
                    tracks
                        .iter()
                        .map(|t| {
                            Related::Uuid(ResourceIdentifier {
                                r#type: ResourceType::Track,
                                id: t.id,
                                meta: None,
                            })
                        })
                        .collect(),
                ),
            },
        );
    }

    MediumResource {
        r#type: ResourceType::Medium,
        id: entity.id,
        attributes: MediumAttributes {
            position: entity.position,
            tracks: entity.tracks,
            track_offset: entity.track_offset,
            format: entity.format.to_owned(),
        },
        meta: None,
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Medium, related: &MediumRelated) -> Included {
    Included::Medium(entity_to_resource(entity, related))
}

fn map_to_tracks_include(include: &[MediumInclude]) -> Vec<TrackInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            MediumInclude::TracksArtists => Some(TrackInclude::Artists),
            _ => None,
        })
        .collect()
}

fn map_to_release_include(include: &[MediumInclude]) -> Vec<ReleaseInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            MediumInclude::ReleaseArtists => Some(ReleaseInclude::Artists),
            _ => None,
        })
        .collect()
}

#[async_recursion]
pub async fn included<C>(
    db: &C,
    related: Vec<MediumRelated>,
    include: &[MediumInclude],
) -> Result<Vec<Included>, TempoError>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&MediumInclude::Release) {
        let releases = related
            .iter()
            .flat_map(|rel| rel.release.clone())
            .collect::<Vec<_>>();
        let releases_related = releases::related(db, &releases, true).await?;
        for (i, release) in releases.into_iter().enumerate() {
            included.push(releases::entity_to_included(&release, &releases_related[i]));
        }
        let releases_included = map_to_release_include(include);
        included.extend(releases::included(db, releases_related, &releases_included).await?);
    }
    if include.contains(&MediumInclude::Tracks) {
        let tracks = related
            .iter()
            .flat_map(|rel| rel.tracks.clone())
            .collect::<Vec<_>>();
        let tracks_related = tracks::related(db, &tracks, true).await?;
        for (i, track) in tracks.into_iter().enumerate() {
            included.push(tracks::entity_to_included(&track, &tracks_related[i]));
        }
        let tracks_included = map_to_tracks_include(include);
        included.extend(tracks::included(db, tracks_related, &tracks_included).await?);
    }
    Ok(included)
}

pub async fn mediums(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<MediumFilter, entity::MediumColumn, MediumInclude, uuid::Uuid>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<MediumResource, Included>>, TempoError> {
    let tx = db.begin().await?;

    let mut mediums_query = entity::MediumEntity::find();
    for (filter_key, filter_value) in opts.filter.iter() {
        if let Some(k) = filter_key.column() {
            mediums_query = mediums_query.filter(ColumnTrait::eq(&k, filter_value.to_owned()));
        }
    }
    for (sort_key, sort_order) in opts.sort.iter() {
        mediums_query = mediums_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    let mut _mediums_cursor = mediums_query.cursor_by(entity::MediumColumn::Id);
    let mediums_cursor = make_cursor(&mut _mediums_cursor, &opts.page);
    let mediums = mediums_cursor.all(&tx).await?;
    let related_to_mediums = related(&tx, &mediums, false).await?;
    let mut data = Vec::new();
    for (i, medium) in mediums.iter().enumerate() {
        data.push(entity_to_resource(medium, &related_to_mediums[i]));
    }
    let included = included(&tx, related_to_mediums, &opts.include).await?;
    Ok(Json(Document {
        links: links_from_resource(&data, opts, &uri),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

pub async fn medium(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<MediumFilter, entity::MediumColumn, MediumInclude, uuid::Uuid>,
) -> Result<Json<Document<MediumResource, Included>>, TempoError> {
    let tx = db.begin().await?;

    let medium = entity::MediumEntity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or(TempoError::NotFound(None))?;
    let related_to_mediums = related(&tx, &[medium.clone()], false).await?;
    let empty_relationship = MediumRelated::default();
    let related = related_to_mediums.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&medium, related);
    let included = included(&tx, related_to_mediums, &opts.include).await?;
    Ok(Json(Document {
        data: DocumentData::Single(data),
        included: dedup(included),
        links: HashMap::new(),
    }))
}
