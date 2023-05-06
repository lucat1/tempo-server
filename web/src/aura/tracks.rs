use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::{body::Body, response::IntoResponse, Json};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, LoaderTrait, ModelTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::images;
use super::releases;
use super::AppState;
use crate::documents::{
    ArtistCreditAttributes, ArtistInclude, ArtistRelation, RecordingAttributes, TrackAttributes,
    TrackRelation,
};
use crate::jsonapi::{
    ArtistResource, Document, DocumentData, Error, Included, Meta, Query, Related, Relation,
    Relationship, ResourceIdentifier, ResourceType, TrackResource,
};

pub struct TrackRelated {
    artist_credits: Vec<entity::ArtistCredit>,
    artists: Vec<Option<entity::Artist>>,
    medium: entity::Medium,
    recorders: Vec<entity::ArtistTrackRelation>,
}

pub async fn related<C>(db: &C, entities: &Vec<entity::Track>) -> Result<Vec<TrackRelated>, DbErr>
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
    let recorders = entities
        .load_many(entity::ArtistTrackRelationEntity, db)
        .await?;

    let mut related = Vec::new();
    for (i, medium) in mediums.into_iter().enumerate() {
        let artist_credits = &artist_credits[i];
        let artists = artist_credits.load_one(entity::ArtistEntity, db).await?;

        let medium = medium.clone().ok_or(DbErr::RecordNotFound(
            "Track doesn't have an associated medium".to_string(),
        ))?;

        related.push(TrackRelated {
            artist_credits: artist_credits.to_owned(),
            artists,
            medium,
            recorders: recorders[i].to_owned(),
        });
    }

    Ok(related)
}

pub fn entity_to_resource(entity: &entity::Track, related: &TrackRelated) -> TrackResource {
    let TrackRelated {
        artist_credits,
        artists,
        medium,
        recorders,
    } = related;
    let mut relationships = HashMap::new();
    let mut related_artists = Vec::new();
    for (i, ac) in artist_credits.into_iter().enumerate() {
        if let Some(artist) = &artists[i] {
            related_artists.push(Related::Artist(ResourceIdentifier {
                r#type: ResourceType::Artist,
                id: artist.id.to_owned(),
                meta: Some(Meta::ArtistCredit(ArtistCreditAttributes {
                    join_phrase: ac.join_phrase.to_owned(),
                })),
            }));
        }
    }
    if !related_artists.is_empty() {
        relationships.insert(
            TrackRelation::Artists,
            Relationship {
                data: Relation::Multi(related_artists),
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
    relationships.insert(
        TrackRelation::Medium,
        Relationship {
            data: Relation::Single(Related::Medium(ResourceIdentifier {
                r#type: ResourceType::Medium,
                id: medium.id,
                meta: None,
            })),
        },
    );

    TrackResource {
        r#type: ResourceType::Track,
        id: entity.id,
        attributes: TrackAttributes {
            title: entity.title.to_owned(),
            track: entity.number,
            disc: medium.position,
            genres: entity.genres.0.to_owned(),
            bpm: None,

            recording_mbid: entity.recording_id.to_owned(),
            track_mbid: entity.id,
            comments: None,

            mimetype: entity.format.unwrap().mime().to_string(),
            duration: None,
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

// pub async fn tracks(
//     State(AppState(db)): State<AppState>,
//     Query(parameters): Query<QueryParameters>,
// ) -> Result<Response, Error> {
//     let tx = db.begin().await.map_err(|e| {
//         Error(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Couldn't begin database transaction".to_string(),
//             e.into(),
//         )
//     })?;
//     let tracks = entity::TrackEntity::find().all(&tx).await.map_err(|e| {
//         Error(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Could not fetch tracks".to_string(),
//             e.into(),
//         )
//     })?;
//     let r = find_related_to_tracks(&tx, tracks).await.map_err(|e| {
//         Error(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Could not fetch entites related to the tracks".to_string(),
//             e.into(),
//         )
//     })?;
//     let tracks = related_to_tracks(&r).map_err(|e| {
//         Error(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Could not aggregate relation data".to_string(),
//             e.into(),
//         )
//     })?;
//
//     let mut doc = vec_to_jsonapi_document(tracks);
//     dedup_document(&mut doc);
//     filter_included(
//         &mut doc,
//         parameters
//             .include
//             .map_or(Vec::new(), |s| s.split(",").map(|s| s.to_owned()).collect()),
//     );
//     Ok(Response(doc))
// }
//
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
//
// pub async fn track(
//     State(AppState(db)): State<AppState>,
//     Path(id): Path<Uuid>,
//     Query(parameters): Query<QueryParameters>,
// ) -> Result<Response, Error> {
//     let tx = db.begin().await.map_err(|e| {
//         Error(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Couldn't begin database transaction".to_string(),
//             e.into(),
//         )
//     })?;
//     let track = find_track_by_id(&tx, id).await?;
//     let RelatedToTracks(artists, mediums, releases, tracks) =
//         find_related_to_tracks(&tx, vec![track])
//             .await
//             .map_err(|e| {
//                 Error(
//                     StatusCode::INTERNAL_SERVER_ERROR,
//                     "Could not fetch entites related to the track".to_string(),
//                     e.into(),
//                 )
//             })?;
//     let (track, track_artist_credits, artist_relations) = tracks.first().unwrap();
//     let track = related_to_track(
//         track,
//         track_artist_credits,
//         artist_relations,
//         &artists,
//         &mediums,
//         &releases,
//     )
//     .map_err(|e| {
//         Error(
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Could not aggregate relation data".to_string(),
//             e.into(),
//         )
//     })?;
//
//     let mut doc = track.to_jsonapi_document();
//     dedup_document(&mut doc);
//     filter_included(
//         &mut doc,
//         parameters
//             .include
//             .map_or(Vec::new(), |s| s.split(",").map(|s| s.to_owned()).collect()),
//     );
//     Ok(Response(doc))
// }

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
