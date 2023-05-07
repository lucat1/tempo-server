use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use uuid::Uuid;

use super::{images, releases, tracks, AppState};
use crate::documents::{
    ArtistAttributes, ArtistCreditAttributes, ArtistInclude, ArtistRelation, RecordingAttributes,
};
use crate::jsonapi::{
    ArtistResource, Document, DocumentData, Error, Included, Meta, Query, Related, Relation,
    Relationship, ResourceIdentifier, ResourceType,
};

#[derive(Default)]
pub struct ArtistRelated {
    images: Vec<entity::ImageArtist>,
    recordings: Vec<entity::ArtistTrackRelation>,
    artist_credits: Vec<entity::ArtistCredit>,
    releases: Vec<Vec<entity::ArtistCreditRelease>>,
    tracks: Vec<Vec<entity::ArtistCreditTrack>>,
}

pub async fn related<C>(db: &C, entities: &Vec<entity::Artist>) -> Result<Vec<ArtistRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let artist_credits = entities.load_many(entity::ArtistCreditEntity, db).await?;
    let images = entities.load_many(entity::ImageArtistEntity, db).await?;
    let recordings = entities
        .load_many(entity::ArtistTrackRelationEntity, db)
        .await?;

    let mut related = Vec::new();
    for i in 0..entities.len() {
        let artist_credits = &artist_credits[i];
        let releases = artist_credits
            .load_many(entity::ArtistCreditReleaseEntity, db)
            .await?;
        let tracks = artist_credits
            .load_many(entity::ArtistCreditTrackEntity, db)
            .await?;

        related.push(ArtistRelated {
            artist_credits: artist_credits.to_owned(),
            images: images[i].to_owned(),
            releases,
            tracks,
            recordings: recordings[i].to_owned(),
        });
    }

    Ok(related)
}

pub async fn included<C>(
    db: &C,
    related: Vec<ArtistRelated>,
    include: Vec<ArtistInclude>,
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&ArtistInclude::Images) {
        let all_artist_images = related
            .iter()
            .map(|rel| rel.images.clone())
            .flatten()
            .collect::<Vec<_>>();
        let images = all_artist_images.load_one(entity::ImageEntity, db).await?;
        included.extend(
            images
                .iter()
                .filter_map(|i| i.as_ref().map(images::entity_to_included))
                .collect::<Vec<_>>(),
        );
    }
    if include.contains(&ArtistInclude::Releases) {
        let release_relations = related
            .iter()
            .map(|rel| rel.releases.to_owned())
            .flatten()
            .flatten()
            .collect::<Vec<_>>();
        let releases = release_relations
            .load_one(entity::ReleaseEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let release_related = releases::related(db, &releases).await?;
        for (i, release) in releases.iter().enumerate() {
            included.push(releases::entity_to_included(release, &release_related[i]))
        }
    }
    if include.contains(&ArtistInclude::Tracks) {
        let track_relations = related
            .into_iter()
            .map(|rel| rel.tracks)
            .flatten()
            .flatten()
            .collect::<Vec<_>>();
        let tracks = track_relations
            .load_one(entity::TrackEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let track_related = tracks::related(db, &tracks).await?;
        for (i, track) in tracks.iter().enumerate() {
            included.push(tracks::entity_to_included(track, &track_related[i]))
        }
    }
    Ok(included)
}

pub fn entity_to_resource(entity: &entity::Artist, related: &ArtistRelated) -> ArtistResource {
    let ArtistRelated {
        images,
        recordings,
        artist_credits,
        releases,
        tracks,
    } = related;
    let mut relationships = HashMap::new();
    if !images.is_empty() {
        relationships.insert(
            ArtistRelation::Images,
            Relationship {
                data: Relation::Multi(
                    images
                        .iter()
                        .map(|i| {
                            Related::Image(ResourceIdentifier {
                                r#type: ResourceType::Image,
                                id: i.image_id.to_owned(),
                                meta: None,
                            })
                        })
                        .collect(),
                ),
            },
        );
    }
    if !recordings.is_empty() {
        relationships.insert(
            ArtistRelation::Recordings,
            Relationship {
                data: Relation::Multi(
                    recordings
                        .iter()
                        .map(|r| {
                            Related::Track(ResourceIdentifier {
                                r#type: ResourceType::Track,
                                id: r.track_id,
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
    let mut related_releases = Vec::new();
    let mut related_tracks = Vec::new();
    for (i, ac) in artist_credits.into_iter().enumerate() {
        related_releases.extend(releases[i].iter().map(|r| {
            Related::Release(ResourceIdentifier {
                r#type: ResourceType::Release,
                id: r.release_id.to_owned(),
                meta: Some(Meta::ArtistCredit(ArtistCreditAttributes {
                    join_phrase: ac.join_phrase.to_owned(),
                })),
            })
        }));
        related_tracks.extend(tracks[i].iter().map(|r| {
            Related::Track(ResourceIdentifier {
                r#type: ResourceType::Track,
                id: r.track_id.to_owned(),
                meta: Some(Meta::ArtistCredit(ArtistCreditAttributes {
                    join_phrase: ac.join_phrase.to_owned(),
                })),
            })
        }));
    }
    if !related_releases.is_empty() {
        relationships.insert(
            ArtistRelation::Releases,
            Relationship {
                data: Relation::Multi(related_releases),
            },
        );
    }
    if !related_tracks.is_empty() {
        relationships.insert(
            ArtistRelation::Tracks,
            Relationship {
                data: Relation::Multi(related_tracks),
            },
        );
    }

    ArtistResource {
        r#type: ResourceType::Artist,
        id: entity.id,
        attributes: ArtistAttributes {
            name: entity.name.to_owned(),
            sort_name: entity.sort_name.to_owned(),
        },
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Artist, related: &ArtistRelated) -> Included {
    Included::Artist(entity_to_resource(entity, related))
}

pub async fn artists(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::ArtistColumn, ArtistInclude>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let mut artists_query = entity::ArtistEntity::find();
    for (sort_key, sort_order) in opts.sort.into_iter() {
        artists_query = artists_query.order_by(sort_key, sort_order);
    }
    for (filter_key, filter_value) in opts.filter.into_iter() {
        artists_query = artists_query.filter(ColumnTrait::eq(&filter_key, filter_value));
    }
    let artists = artists_query.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all artists".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_artists = related(&tx, &artists).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch entites related to the artists".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, artist) in artists.iter().enumerate() {
        data.push(entity_to_resource(artist, &related_to_artists[i]));
    }
    let included = included(&tx, related_to_artists, opts.include)
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

pub async fn artist(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<entity::ArtistColumn, ArtistInclude>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let artist = entity::ArtistEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried artist".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Artist not found".to_string(),
            detail: None,
        })?;
    let related_to_artists = related(&tx, &vec![artist.clone()])
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the artists".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = ArtistRelated::default();
    let related = related_to_artists.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&artist, related);
    let included = included(&tx, related_to_artists, opts.include)
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
