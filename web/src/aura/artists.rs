use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use uuid::Uuid;

use super::artist_credits;
use super::AppState;
use crate::documents::{ArtistAttributes, ArtistRelation};
use crate::jsonapi::{
    ArtistResource, Document, DocumentData, Error, Included, Query, Related, Relation,
    RelationData, Relationship, ResourceType,
};

async fn related<C>(
    db: &C,
    entities: &Vec<entity::Artist>,
) -> Result<Vec<(Vec<entity::ImageArtist>, Vec<entity::ArtistCredit>)>, DbErr>
where
    C: ConnectionTrait,
{
    // let images = entity::ImageArtistEntity::find()
    //     .filter(entity::ImageArtistColumn::ArtistId.is_in(entities.iter().map(|a| a.id)))
    //     .all(db)
    //     .await?;
    let images = entities.load_many(entity::ImageArtistEntity, db).await?;
    let artist_credits = entities.load_many(entity::ArtistCreditEntity, db).await?;
    let mut results = Vec::new();
    for i in 0..images.len() {
        results.push((images[i].to_owned(), artist_credits[i].to_owned()));
    }
    Ok(results)
}

pub fn entity_to_resource(
    entity: &entity::Artist,
    related_images: &Vec<entity::ImageArtist>,
    related_artist_credits: &Vec<entity::ArtistCredit>,
) -> ArtistResource {
    ArtistResource {
        r#type: ResourceType::Artist,
        id: entity.id,
        attributes: ArtistAttributes {
            name: entity.name.to_owned(),
            sort_name: entity.sort_name.to_owned(),
        },
        relationships: [
            (
                ArtistRelation::Image,
                Relationship {
                    data: Relation::Multi(
                        related_images
                            .iter()
                            .map(|i| {
                                Related::Image(RelationData {
                                    r#type: ResourceType::Image,
                                    id: i.image_id.to_owned(),
                                })
                            })
                            .collect(),
                    ),
                },
            ),
            (
                ArtistRelation::ArtistCredit,
                Relationship {
                    data: Relation::Multi(
                        related_artist_credits
                            .iter()
                            .map(|a| {
                                Related::ArtistCredit(RelationData {
                                    r#type: ResourceType::ArtistCredit,
                                    id: a.id.to_owned(),
                                })
                            })
                            .collect(),
                    ),
                },
            ),
        ]
        .into(),
    }
}

pub fn entity_to_included(
    entity: &entity::Artist,
    related_images: &Vec<entity::ImageArtist>,
    related_artist_credits: &Vec<entity::ArtistCredit>,
) -> Included {
    Included::Artist(entity_to_resource(entity,related_images,related_artist_credits))
}

pub async fn artists(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::ArtistColumn>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't being database transaction".to_string(),
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
    let mut included = Vec::new();
    for (i, artist) in artists.iter().enumerate() {
        let (related_images, related_artist_credits) = &related_to_artists[i];
        data.push(entity_to_resource(
            artist,
            related_images,
            related_artist_credits,
        ));
        if opts.include.contains(&ResourceType::ArtistCredit) {
            println!("adding credits");
            included.extend(
                related_artist_credits
                    .iter()
                    .map(artist_credits::entity_to_included)
                    .collect::<Vec<Included>>(),
            )
        }
    }
    Ok(Json(Document {
        data: DocumentData::Multi(data),
        included,
    }))
}

pub async fn artist(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Document<ArtistResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't being database transaction".to_string(),
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
    let empty_relationship = (Vec::new(), Vec::new());
    let (related_images, related_artist_credits) =
        related_to_artists.first().unwrap_or(&empty_relationship);
    let data = entity_to_resource(&artist, related_images, related_artist_credits);
    Ok(Json(Document {
        data: DocumentData::Single(data),
        included: vec![],
    }))
}
