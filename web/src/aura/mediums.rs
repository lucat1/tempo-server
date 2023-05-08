use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use uuid::Uuid;

use super::{releases, tracks, AppState};
use crate::documents::{MediumAttributes, MediumInclude, MediumRelation};
use crate::jsonapi::{
    Document, DocumentData, Error, Included, MediumResource, Query, Related, Relation,
    Relationship, ResourceIdentifier, ResourceType,
};

#[derive(Default)]
pub struct MediumRelated {
    release: Option<entity::Release>,
    tracks: Vec<entity::Track>,
}

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::Medium>,
    _light: bool,
) -> Result<Vec<MediumRelated>, DbErr>
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
                data: Relation::Single(Related::Release(ResourceIdentifier {
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
                        .into_iter()
                        .map(|t| {
                            Related::Track(ResourceIdentifier {
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
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Medium, related: &MediumRelated) -> Included {
    Included::Medium(entity_to_resource(entity, related))
}

pub async fn included<C>(
    db: &C,
    related: Vec<MediumRelated>,
    include: Vec<MediumInclude>,
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&MediumInclude::Release) {
        let releases = related
            .iter()
            .map(|rel| rel.release.clone())
            .flatten()
            .collect::<Vec<_>>();
        let releases_related = releases::related(db, &releases, true).await?;
        for (i, release) in releases.into_iter().enumerate() {
            included.push(releases::entity_to_included(&release, &releases_related[i]));
        }
    }
    if include.contains(&MediumInclude::Tracks) {
        let tracks = related
            .iter()
            .map(|rel| rel.tracks.clone())
            .flatten()
            .collect::<Vec<_>>();
        let tracks_related = tracks::related(db, &tracks, true).await?;
        for (i, track) in tracks.into_iter().enumerate() {
            included.push(tracks::entity_to_included(&track, &tracks_related[i]));
        }
    }
    Ok(included)
}

pub async fn mediums(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::MediumColumn, MediumInclude>,
) -> Result<Json<Document<MediumResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let mut mediums_query = entity::MediumEntity::find();
    for (sort_key, sort_order) in opts.sort.into_iter() {
        mediums_query = mediums_query.order_by(sort_key, sort_order);
    }
    for (filter_key, filter_value) in opts.filter.into_iter() {
        mediums_query = mediums_query.filter(ColumnTrait::eq(&filter_key, filter_value));
    }
    let mediums = mediums_query.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all releases".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_mediums = related(&tx, &mediums, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch entites related to the releases".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, medium) in mediums.iter().enumerate() {
        data.push(entity_to_resource(medium, &related_to_mediums[i]));
    }
    let included = included(&tx, related_to_mediums, opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json(Document {
        data: DocumentData::Multi(data),
        included,
    }))
}

pub async fn medium(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<entity::MediumColumn, MediumInclude>,
) -> Result<Json<Document<MediumResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let medium = entity::MediumEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried medium".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Medium not found".to_string(),
            detail: None,
        })?;
    let related_to_mediums = related(&tx, &vec![medium.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the mediums".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = MediumRelated::default();
    let related = related_to_mediums.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&medium, related);
    let included = included(&tx, related_to_mediums, opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json(Document {
        data: DocumentData::Single(data),
        included,
    }))
}
