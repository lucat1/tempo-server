use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::{body::Body, response::IntoResponse, Json};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::{artists, mediums, AppState};
use crate::documents::{
    ArtistCreditAttributes, RecordingAttributes, TrackAttributes, TrackInclude, TrackRelation,
};
use crate::jsonapi::{
    Document, DocumentData, Error, Included, Meta, Query, Related, Relation, Relationship,
    ResourceIdentifier, ResourceType, TrackResource,
};

#[derive(Default)]
pub struct TrackRelated {
    artist_credits: Vec<entity::ArtistCredit>,
    medium: Option<entity::Medium>,
    recorders: Vec<entity::ArtistTrackRelation>,
}

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::Track>,
    light: bool,
) -> Result<Vec<TrackRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let artist_credits = entities
        .load_many_to_many(
            entity::ArtistCreditEntity,
            entity::ArtistCreditTrackEntity,
            db,
        )
        .await?;
    let mediums = entities.load_one(entity::MediumEntity, db).await?;
    let recorders = if !light {
        entities
            .load_many(entity::ArtistTrackRelationEntity, db)
            .await?
    } else {
        Vec::new()
    };

    let mut related = Vec::new();
    for (i, medium) in mediums.into_iter().enumerate() {
        let artist_credits = &artist_credits[i];

        related.push(TrackRelated {
            artist_credits: artist_credits.to_owned(),
            medium,
            recorders: if light {
                Vec::new()
            } else {
                recorders[i].to_owned()
            },
        });
    }

    Ok(related)
}

pub fn entity_to_resource(entity: &entity::Track, related: &TrackRelated) -> TrackResource {
    let TrackRelated {
        artist_credits,
        medium,
        recorders,
    } = related;
    let mut relationships = HashMap::new();
    if !artist_credits.is_empty() {
        relationships.insert(
            TrackRelation::Artists,
            Relationship {
                data: Relation::Multi(
                    artist_credits
                        .into_iter()
                        .map(|ac| {
                            Related::Artist(ResourceIdentifier {
                                r#type: ResourceType::Artist,
                                id: ac.artist_id.to_owned(),
                                meta: Some(Meta::ArtistCredit(ArtistCreditAttributes {
                                    join_phrase: ac.join_phrase.to_owned(),
                                })),
                            })
                        })
                        .collect(),
                ),
            },
        );
    }
    if !recorders.is_empty() {
        relationships.insert(
            TrackRelation::Recorders,
            Relationship {
                data: Relation::Multi(
                    recorders
                        .iter()
                        .map(|r| {
                            Related::Artist(ResourceIdentifier {
                                r#type: ResourceType::Track,
                                id: r.artist_id,
                                meta: Some(Meta::Recording(RecordingAttributes {
                                    role: r.relation_type,
                                    detail: r.relation_value.to_owned(),
                                })),
                            })
                        })
                        .collect(),
                ),
            },
        );
    }
    if let Some(med) = medium {
        relationships.insert(
            TrackRelation::Medium,
            Relationship {
                data: Relation::Single(Related::Medium(ResourceIdentifier {
                    r#type: ResourceType::Medium,
                    id: med.id,
                    meta: None,
                })),
            },
        );
    }

    TrackResource {
        r#type: ResourceType::Track,
        id: entity.id,
        attributes: TrackAttributes {
            title: entity.title.to_owned(),
            track: entity.number,
            disc: medium.as_ref().map(|m| m.position),
            genres: entity.genres.0.to_owned(),
            bpm: None,

            recording_mbid: entity.recording_id.to_owned(),
            track_mbid: entity.id,
            comments: None,

            mimetype: entity.format.unwrap().mime().to_string(),
            duration: entity.length,
            framerate: None,
            framecount: None,
            channels: None,
            bitrate: None,
            bitdepth: None,
            size: None, // TODO
        },
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Track, related: &TrackRelated) -> Included {
    Included::Track(entity_to_resource(entity, related))
}

pub async fn included<C>(
    db: &C,
    related: Vec<TrackRelated>,
    include: Vec<TrackInclude>,
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&TrackInclude::Artists) {
        let artist_credits = related
            .iter()
            .map(|rel| rel.artist_credits.clone())
            .flatten()
            .collect::<Vec<_>>();
        let artists = artist_credits
            .load_one(entity::ArtistEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();
        let artists_related = artists::related(db, &artists, true).await?;
        for (i, artist) in artists.into_iter().enumerate() {
            included.push(artists::entity_to_included(&artist, &artists_related[i]));
        }
    }
    if include.contains(&TrackInclude::Medium) {
        let mediums = related
            .iter()
            .filter_map(|rel| rel.medium.clone())
            .collect::<Vec<_>>();
        let mediums_related = mediums::related(db, &mediums, true).await?;
        for (i, medium) in mediums.into_iter().enumerate() {
            included.push(mediums::entity_to_included(&medium, &mediums_related[i]));
        }
    }
    if include.contains(&TrackInclude::Recorders) {
        let recorders = related
            .iter()
            .map(|rel| rel.recorders.clone())
            .flatten()
            .collect::<Vec<_>>();
        let artists = recorders
            .load_one(entity::ArtistEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();
        let artists_related = artists::related(db, &artists, true).await?;
        for (i, artist) in artists.into_iter().enumerate() {
            included.push(artists::entity_to_included(&artist, &artists_related[i]));
        }
    }
    Ok(included)
}

async fn find_track_by_id<C>(db: &C, id: Uuid) -> Result<entity::Track, Error>
where
    C: ConnectionTrait,
{
    entity::TrackEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch track".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Not found".to_string(),
            detail: Some("Not found".into()),
        })
}

pub async fn tracks(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::TrackColumn, TrackInclude>,
) -> Result<Json<Document<TrackResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let mut tracks_query = entity::TrackEntity::find();
    for (sort_key, sort_order) in opts.sort.into_iter() {
        tracks_query = tracks_query.order_by(sort_key, sort_order);
    }
    for (filter_key, filter_value) in opts.filter.into_iter() {
        tracks_query = tracks_query.filter(ColumnTrait::eq(&filter_key, filter_value));
    }
    let tracks = tracks_query.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all tracks".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_tracks = related(&tx, &tracks, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch entites related to the tracks".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, track) in tracks.iter().enumerate() {
        data.push(entity_to_resource(track, &related_to_tracks[i]));
    }
    let included = included(&tx, related_to_tracks, opts.include)
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

pub async fn track(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<entity::TrackColumn, TrackInclude>,
) -> Result<Json<Document<TrackResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let track = find_track_by_id(&tx, id).await?;
    let related_to_tracks = related(&tx, &vec![track.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the track".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = TrackRelated::default();
    let related = related_to_tracks.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&track, related);
    let included = included(&tx, related_to_tracks, opts.include)
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

pub async fn audio(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request<Body>,
) -> Result<impl IntoResponse, Error> {
    let track = find_track_by_id(&db, id).await?;
    let path = track.path.ok_or(Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Track does not have an associated path".to_string(),
        detail: Some("Track does not have an associated path".into()),
    })?;
    let mime = track
        .format
        .ok_or(Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Track does not have an associated format".to_string(),
            detail: Some("Track does not have an associated format".into()),
        })?
        .mime();
    Ok(
        tower_http::services::fs::ServeFile::new_with_mime(path, &mime)
            .oneshot(request)
            .await,
    )
}
