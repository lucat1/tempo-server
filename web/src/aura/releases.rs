use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Datelike;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use uuid::Uuid;

use super::{artists, images, AppState};
use crate::documents::{
    ArtistCreditAttributes, ReleaseAttributes, ReleaseInclude, ReleaseRelation,
};
use crate::jsonapi::{
    Document, DocumentData, Error, Included, Meta, Query, Related, Relation, Relationship,
    ReleaseResource, ResourceIdentifier, ResourceType,
};

#[derive(Default)]
pub struct ReleaseRelated {
    image: Option<entity::ImageRelease>,
    artist_credits: Vec<entity::ArtistCredit>,
    artists: Vec<Option<entity::Artist>>,
    mediums: Vec<entity::Medium>,
}

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::Release>,
) -> Result<Vec<ReleaseRelated>, DbErr>
where
    C: ConnectionTrait,
{
    let artist_credits = entities
        .load_many_to_many(
            entity::ArtistCreditEntity,
            entity::ArtistCreditReleaseEntity,
            db,
        )
        .await?;
    let images = entities.load_one(entity::ImageReleaseEntity, db).await?;
    let mediums = entities.load_many(entity::MediumEntity, db).await?;

    let mut related = Vec::new();
    for i in 0..entities.len() {
        let artist_credits = &artist_credits[i];
        let artists = artist_credits.load_one(entity::ArtistEntity, db).await?;

        related.push(ReleaseRelated {
            image: images[i].to_owned(),
            artist_credits: artist_credits.to_owned(),
            artists,
            mediums: mediums[i].to_owned(),
        });
    }

    Ok(related)
}

pub fn entity_to_resource(entity: &entity::Release, related: &ReleaseRelated) -> ReleaseResource {
    let ReleaseRelated {
        image,
        artist_credits,
        artists,
        mediums,
    } = related;
    let mut relationships = HashMap::new();
    if let Some(img) = image {
        relationships.insert(
            ReleaseRelation::Image,
            Relationship {
                data: Relation::Single(Related::Image(ResourceIdentifier {
                    r#type: ResourceType::Image,
                    id: img.image_id.to_owned(),
                    meta: None,
                })),
            },
        );
    }
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
            ReleaseRelation::Artists,
            Relationship {
                data: Relation::Multi(related_artists),
            },
        );
    }
    if !mediums.is_empty() {
        relationships.insert(
            ReleaseRelation::Mediums,
            Relationship {
                data: Relation::Multi(
                    mediums
                        .iter()
                        .map(|m| {
                            Related::Medium(ResourceIdentifier {
                                r#type: ResourceType::Medium,
                                id: m.id,
                                meta: None,
                            })
                        })
                        .collect(),
                ),
            },
        );
    }

    ReleaseResource {
        r#type: ResourceType::Release,
        id: entity.id,
        attributes: ReleaseAttributes {
            title: entity.title.to_owned(),
            disctotal: mediums.len() as u32,
            tracktotal: mediums.iter().fold(0, |acc, m| acc + m.tracks),
            genres: entity.genres.0.to_owned(),

            year: entity.date.map(|d| d.year()),
            month: entity.date.map(|d| d.month()),
            day: entity.date.map(|d| d.day()),
            original_year: entity.original_date.map(|d| d.year()),
            original_month: entity.original_date.map(|d| d.month()),
            original_day: entity.original_date.map(|d| d.day()),

            release_type: entity.release_type.to_owned(),
            release_mbid: entity.id,
            release_group_mbid: entity.release_group_id,
        },
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Release, related: &ReleaseRelated) -> Included {
    Included::Release(entity_to_resource(entity, related))
}

pub async fn included<C>(
    db: &C,
    related: Vec<ReleaseRelated>,
    include: Vec<ReleaseInclude>,
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&ReleaseInclude::Image) {
        let all_release_images = related
            .iter()
            .map(|rel| rel.image.clone())
            .flatten()
            .collect::<Vec<_>>();
        let images = all_release_images.load_one(entity::ImageEntity, db).await?;
        included.extend(
            images
                .iter()
                .filter_map(|i| i.as_ref().map(images::entity_to_included))
                .collect::<Vec<_>>(),
        );
    }
    if include.contains(&ReleaseInclude::Artists) {
        let artist_credits = related
            .iter()
            .map(|rel| rel.artist_credits.to_owned())
            .flatten()
            .collect::<Vec<_>>();
        let artists = artist_credits
            .load_one(entity::ArtistEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let artists_related = artists::related(db, &artists).await?;
        for (i, artist) in artists.into_iter().enumerate() {
            included.push(artists::entity_to_included(&artist, &artists_related[i]));
        }
    }
    if include.contains(&ReleaseInclude::Mediums) {
        // TODO
    }
    Ok(included)
}

pub async fn releases(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::ReleaseColumn, ReleaseInclude>,
) -> Result<Json<Document<ReleaseResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let mut releases_query = entity::ReleaseEntity::find();
    for (sort_key, sort_order) in opts.sort.into_iter() {
        releases_query = releases_query.order_by(sort_key, sort_order);
    }
    for (filter_key, filter_value) in opts.filter.into_iter() {
        releases_query = releases_query.filter(ColumnTrait::eq(&filter_key, filter_value));
    }
    let releases = releases_query.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all releases".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_releases = related(&tx, &releases).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch entites related to the releases".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, release) in releases.iter().enumerate() {
        data.push(entity_to_resource(release, &related_to_releases[i]));
    }
    let included = included(&tx, related_to_releases, opts.include)
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

pub async fn release(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<entity::ReleaseColumn, ReleaseInclude>,
) -> Result<Json<Document<ReleaseResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let release = entity::ReleaseEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried release".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Release not found".to_string(),
            detail: None,
        })?;
    let related_to_releases = related(&tx, &vec![release.clone()])
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the releases".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = ReleaseRelated::default();
    let related = related_to_releases.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&release, related);
    let included = included(&tx, related_to_releases, opts.include)
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
