use std::collections::HashMap;

use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use sea_orm::{
    ColumnTrait, ConnectionTrait, CursorTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use uuid::Uuid;

use super::{artists, images, mediums};
use crate::api::{
    documents::{
        ArtistCreditAttributes, MediumInclude, ReleaseAttributes, ReleaseInclude, ReleaseRelation,
    },
    extract::Json,
    jsonapi::{
        dedup, links_from_resource, make_cursor, Document, DocumentData, Error, Included, Meta,
        Query, Related, Relation, Relationship, ReleaseResource, ResourceIdentifier, ResourceType,
    },
    AppState,
};

#[derive(Default)]
pub struct ReleaseRelated {
    image: Option<entity::ImageRelease>,
    artist_credits: Vec<entity::ArtistCredit>,
    mediums: Vec<entity::Medium>,
}

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::Release>,
    _light: bool,
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

        related.push(ReleaseRelated {
            image: images[i].to_owned(),
            artist_credits: artist_credits.to_owned(),
            mediums: mediums[i].to_owned(),
        });
    }

    Ok(related)
}

pub fn entity_to_resource(entity: &entity::Release, related: &ReleaseRelated) -> ReleaseResource {
    let ReleaseRelated {
        image,
        artist_credits,
        mediums,
    } = related;
    let mut relationships = HashMap::new();
    if !artist_credits.is_empty() {
        relationships.insert(
            ReleaseRelation::Artists,
            Relationship {
                data: Relation::Multi(
                    artist_credits
                        .iter()
                        .map(|ac| {
                            Related::Uuid(ResourceIdentifier {
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
    if !mediums.is_empty() {
        relationships.insert(
            ReleaseRelation::Mediums,
            Relationship {
                data: Relation::Multi(
                    mediums
                        .iter()
                        .map(|m| {
                            Related::Uuid(ResourceIdentifier {
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
    if let Some(img) = image {
        relationships.insert(
            ReleaseRelation::Image,
            Relationship {
                data: Relation::Single(Related::String(ResourceIdentifier {
                    r#type: ResourceType::Image,
                    id: img.image_id.to_owned(),
                    meta: None,
                })),
            },
        );
    }

    ReleaseResource {
        r#type: ResourceType::Release,
        id: entity.id,
        attributes: ReleaseAttributes {
            title: entity.title.to_owned(),
            disctotal: mediums.len() as i32,
            tracktotal: mediums.iter().fold(0, |acc, m| acc + m.tracks),
            genres: entity.genres.0.to_owned(),

            year: entity.year,
            month: entity.month,
            day: entity.day,
            original_year: entity.year,
            original_month: entity.month,
            original_day: entity.day,

            release_type: entity.release_type.to_owned(),
            release_mbid: entity.id,
            release_group_mbid: entity.release_group_id,
        },
        meta: HashMap::new(),
        relationships,
    }
}

pub fn entity_to_included(entity: &entity::Release, related: &ReleaseRelated) -> Included {
    Included::Release(entity_to_resource(entity, related))
}

fn map_to_mediums_include(include: &[ReleaseInclude]) -> Vec<MediumInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            ReleaseInclude::MediumsTracks => Some(MediumInclude::Tracks),
            ReleaseInclude::MediumsTracksArtists => Some(MediumInclude::TracksArtists),
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    related: Vec<ReleaseRelated>,
    include: &[ReleaseInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&ReleaseInclude::Image) {
        let all_release_images = related
            .iter()
            .flat_map(|rel| rel.image.clone())
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
            .flat_map(|rel| rel.artist_credits.to_owned())
            .collect::<Vec<_>>();
        let artists = artist_credits
            .load_one(entity::ArtistEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let artists_related = artists::related(db, &artists, true).await?;
        for (i, artist) in artists.into_iter().enumerate() {
            included.push(artists::entity_to_included(&artist, &artists_related[i]));
        }
    }
    if include.contains(&ReleaseInclude::Mediums) {
        let mediums = related
            .iter()
            .flat_map(|rel| rel.mediums.to_owned())
            .collect::<Vec<_>>();
        let mediums_related = mediums::related(db, &mediums, true).await?;
        for (i, medium) in mediums.into_iter().enumerate() {
            included.push(mediums::entity_to_included(&medium, &mediums_related[i]));
        }
        let mediums_included = map_to_mediums_include(include);
        included.extend(mediums::included(db, mediums_related, &mediums_included).await?);
    }
    Ok(included)
}

pub async fn releases(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::ReleaseColumn, ReleaseInclude, uuid::Uuid>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<ReleaseResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;

    let mut releases_query = entity::ReleaseEntity::find();
    for (sort_key, sort_order) in opts.sort.iter() {
        releases_query = releases_query.order_by(sort_key.to_owned(), sort_order.to_owned());
    }
    for (filter_key, filter_value) in opts.filter.iter() {
        releases_query =
            releases_query.filter(ColumnTrait::eq(filter_key, filter_value.to_owned()));
    }
    let mut _releases_cursor = releases_query.cursor_by(entity::ReleaseColumn::Id);
    let releases_cursor = make_cursor(&mut _releases_cursor, &opts.page);
    let releases = releases_cursor.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all releases".to_string(),
        detail: Some(e.into()),
    })?;
    let related_to_releases = related(&tx, &releases, false).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch entites related to the releases".to_string(),
        detail: Some(e.into()),
    })?;
    let mut data = Vec::new();
    for (i, release) in releases.iter().enumerate() {
        data.push(entity_to_resource(release, &related_to_releases[i]));
    }
    let included = included(&tx, related_to_releases, &opts.include)
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

pub async fn release(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(opts): Query<entity::ReleaseColumn, ReleaseInclude, uuid::Uuid>,
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
    let related_to_releases = related(&tx, &vec![release.clone()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the releases".to_string(),
            detail: Some(e.into()),
        })?;
    let empty_relationship = ReleaseRelated::default();
    let related = related_to_releases.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&release, related);
    let included = included(&tx, related_to_releases, &opts.include)
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
