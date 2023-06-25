use std::collections::HashMap;

use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use sea_orm::{
    ColumnTrait, ConnectionTrait, CursorTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use uuid::Uuid;

use super::{images, releases, tracks};
use crate::api::{
    documents::{
        ArtistAttributes, ArtistCreditAttributes, ArtistFilter, ArtistInclude, ArtistRelation,
        IntoColumn, RecordingAttributes, ReleaseInclude,
    },
    extract::Json,
    jsonapi::{
        dedup, links_from_resource, make_cursor, ArtistResource, Document, DocumentData, Error,
        Included, Meta, Query, Related, Relation, Relationship, ResourceIdentifier, ResourceType,
    },
    AppState,
};

#[derive(Default)]
pub struct ArtistRelated {
    relations: Vec<entity::ArtistUrl>,
    images: Vec<entity::ImageArtist>,
    recordings: Vec<entity::ArtistTrackRelation>,
    artist_credits: Vec<entity::ArtistCredit>,
    releases: Vec<Vec<entity::ArtistCreditRelease>>,
    tracks: Vec<Vec<entity::ArtistCreditTrack>>,
}

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::Artist>,
    light: bool,
) -> Result<Vec<ArtistRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let artist_relations = entities.load_many(entity::ArtistUrlEntity, db).await?;
    let artist_credits = entities.load_many(entity::ArtistCreditEntity, db).await?;
    let images = entities.load_many(entity::ImageArtistEntity, db).await?;
    let recordings = if !light {
        entities
            .load_many(entity::ArtistTrackRelationEntity, db)
            .await?
    } else {
        Vec::new()
    };

    let mut related = Vec::new();
    for i in 0..entities.len() {
        let artist_credits = &artist_credits[i];
        let releases = artist_credits
            .load_many(entity::ArtistCreditReleaseEntity, db)
            .await?;
        let tracks = if !light {
            artist_credits
                .load_many(entity::ArtistCreditTrackEntity, db)
                .await?
        } else {
            Vec::new()
        };

        related.push(ArtistRelated {
            relations: artist_relations[i].to_owned(),
            artist_credits: if !light {
                artist_credits.to_owned()
            } else {
                Vec::new()
            },
            images: images[i].to_owned(),
            releases,
            tracks,
            recordings: if light {
                Vec::new()
            } else {
                recordings[i].to_owned()
            },
        });
    }

    Ok(related)
}

fn map_to_releases_include(include: &[ArtistInclude]) -> Vec<ReleaseInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            ArtistInclude::ReleasesArtists => Some(ReleaseInclude::Artists),
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    related: Vec<ArtistRelated>,
    include: &[ArtistInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&ArtistInclude::Images) {
        let all_artist_images = related
            .iter()
            .flat_map(|rel| rel.images.clone())
            .collect::<Vec<_>>();
        let images = all_artist_images.load_one(entity::ImageEntity, db).await?;
        included.extend(
            images
                .iter()
                .filter_map(|i| i.as_ref().map(images::entity_to_included))
                .collect::<Vec<_>>(),
        );
    }
    if include.contains(&ArtistInclude::Tracks) {
        let track_relations = related
            .iter()
            .flat_map(|rel| rel.tracks.to_owned())
            .flatten()
            .collect::<Vec<_>>();
        let tracks = track_relations
            .load_one(entity::TrackEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let track_related = tracks::related(db, &tracks, true).await?;
        for (i, track) in tracks.iter().enumerate() {
            included.push(tracks::entity_to_included(track, &track_related[i]))
        }
    }
    if include.contains(&ArtistInclude::Releases) {
        let release_relations = related
            .into_iter()
            .flat_map(|rel| rel.releases)
            .flatten()
            .collect::<Vec<_>>();
        let releases = release_relations
            .load_one(entity::ReleaseEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let releases_related = releases::related(db, &releases, true).await?;
        for (i, release) in releases.iter().enumerate() {
            included.push(releases::entity_to_included(release, &releases_related[i]))
        }
        let releases_included = map_to_releases_include(include);
        included.extend(releases::included(db, releases_related, &releases_included).await?);
    }
    Ok(included)
}

pub fn entity_to_resource(entity: &entity::Artist, related: &ArtistRelated) -> ArtistResource {
    let ArtistRelated {
        relations,
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
                            Related::String(ResourceIdentifier {
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
                            Related::Uuid(ResourceIdentifier {
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
    for (i, ac) in artist_credits.iter().enumerate() {
        related_releases.extend(releases[i].iter().map(|r| {
            Related::Uuid(ResourceIdentifier {
                r#type: ResourceType::Release,
                id: r.release_id.to_owned(),
                meta: Some(Meta::ArtistCredit(ArtistCreditAttributes {
                    join_phrase: ac.join_phrase.to_owned(),
                })),
            })
        }));
        related_tracks.extend(tracks[i].iter().map(|r| {
            Related::Uuid(ResourceIdentifier {
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
            description: entity.description.to_owned(),
            urls: relations
                .iter()
                .map(|rel| (rel.r#type, rel.url.to_owned()))
                .collect(),
        },
        meta: HashMap::new(),
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Artist, related: &ArtistRelated) -> Included {
    Included::Artist(entity_to_resource(entity, related))
}

pub async fn artists(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ArtistFilter, entity::ArtistColumn, ArtistInclude, uuid::Uuid>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let mut artists_query = entity::ArtistEntity::find();
    for (filter_key, filter_value) in opts.filter.iter() {
        if let Some(k) = filter_key.column() {
            artists_query = artists_query.filter(ColumnTrait::eq(&k, filter_value));
        }
    }
    for (sort_key, sort_order) in opts.sort.iter() {
        artists_query = artists_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    let mut _artists_cursor = artists_query.cursor_by(entity::ArtistColumn::Id);
    let artists_cursor = make_cursor(&mut _artists_cursor, &opts.page);
    let artists = artists_cursor.all(&db).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch artists page".to_string(),
        detail: Some(e.into()),
    })?;

    let related_to_artists = related(&tx, &artists, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch entites related to the artists".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, artist) in artists.iter().enumerate() {
        data.push(entity_to_resource(artist, &related_to_artists[i]));
    }
    let included = included(&tx, related_to_artists, &opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        links: links_from_resource(&data, opts, &uri),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

pub async fn artist(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<ArtistFilter, entity::ArtistColumn, ArtistInclude, uuid::Uuid>,
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
    let related_to_artists = related(&tx, &vec![artist.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the artists".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = ArtistRelated::default();
    let related = related_to_artists.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&artist, related);
    let included = included(&tx, related_to_artists, &opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        data: DocumentData::Single(data),
        included: dedup(included),
        links: HashMap::new(),
    }))
}
