use std::collections::HashMap;

use chrono::Datelike;
use itertools::Itertools;
use sea_orm::{ConnectionTrait, DbErr, LoaderTrait};

use crate::documents::{
    ArtistCreditAttributes, RecordingAttributes, ReleaseAttributes, ReleaseRelation,
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
