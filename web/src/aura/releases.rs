use std::collections::VecDeque;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use eyre::{eyre, Result};
use jsonapi::model::*;
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, ModelTrait, TransactionTrait};
use uuid::Uuid;

use super::documents::{dedup_document, filter_included, Release};
use crate::response::{Error, Response};
use web::{AppState, QueryParameters};

#[derive(Debug)]
struct RelatedToReleases(
    pub HashMap<Uuid, entity::Artist>,
    pub HashMap<Uuid, entity::Medium>,
    pub Vec<(entity::Release, Vec<entity::ArtistCredit>, entity::Image)>,
);

async fn find_related_to_releases<C>(
    db: &C,
    src_releases: Vec<entity::Release>,
) -> Result<RelatedToReleases>
where
    C: ConnectionTrait,
{
    let mut artists = HashMap::new();
    let mut mediums = HashMap::new();
    let mut releases = Vec::new();

    let mut artist_credits: VecDeque<_> = src_releases
        .load_many_to_many(
            entity::ArtistCreditEntity,
            entity::ArtistCreditReleaseEntity,
            db,
        )
        .await?
        .into();
    let artsts: VecDeque<_> = artist_credits
        .clone()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .load_one(entity::ArtistEntity, db)
        .await?
        .into_iter()
        .flatten()
        .collect();
    for artist in artsts {
        artists.insert(artist.id, artist);
    }
    let medms = src_releases
        .load_many(entity::MediumEntity, db)
        .await?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    for medium in medms {
        mediums.insert(medium.id, medium);
    }

    for release in src_releases.into_iter() {
        let image = release
            .find_related(entity::ImageEntity)
            .one(db)
            .await?
            .ok_or(eyre!(
                "Release {} doesn't have an associated image",
                release.id
            ))?;
        releases.push((
            release,
            artist_credits
                .pop_front()
                .ok_or(eyre!("Missing artist credits relations"))?,
            image,
        ))
    }

    Ok(RelatedToReleases(artists, mediums, releases))
}

fn related_to_releases(r: &RelatedToReleases) -> Result<Vec<Release>> {
    let RelatedToReleases(artists, mediums, releases) = r;
    let mut results = vec![];
    for (release, artist_credits, image) in releases.iter() {
        results.push(related_to_release(
            release,
            artist_credits,
            image,
            artists,
            mediums,
        ));
    }
    Ok(results)
}

pub fn related_to_release(
    release: &entity::Release,
    artist_credits: &Vec<entity::ArtistCredit>,
    image: &entity::Image,
    artists: &HashMap<Uuid, entity::Artist>,
    mediums: &HashMap<Uuid, entity::Medium>,
) -> Release {
    let artists = artist_credits
        .iter()
        .filter_map(|ac| {
            artists
                .get(&ac.artist_id)
                .map(|artist| super::artists::artist_credit_to_artist_credit(ac, artist))
        })
        .collect();
    Release {
        id: release.id,
        title: release.title.to_owned(),
        artists,
        disctotal: mediums
            .values()
            .filter(|m| m.release_id == release.id)
            .count() as u32,
        tracktotal: mediums
            .values()
            .filter(|m| m.release_id == release.id)
            .fold(0, |c, m| c + m.tracks) as u32,
        genres: release.genres.0.to_owned(),
        release_mbid: release.id,
        release_group_mbid: release.release_group_id,
        year: release.date.map(|d| d.year()),
        day: release.date.map(|d| d.day()),
        month: release.date.map(|d| d.month()),
        image: super::images::image_to_image(image),
    }
}

pub async fn releases(
    State(AppState(db)): State<AppState>,
    Query(parameters): Query<QueryParameters>,
) -> Result<Response, Error> {
    let tx = db.begin().await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't being database transaction".to_string(),
            e.into(),
        )
    })?;
    let releases = entity::ReleaseEntity::find().all(&tx).await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not fetch releases".to_string(),
            e.into(),
        )
    })?;
    let r = find_related_to_releases(&tx, releases).await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not fetch entites related to the tracks".to_string(),
            e.into(),
        )
    })?;
    let releases = related_to_releases(&r).map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not aggregate relation data".to_string(),
            e.into(),
        )
    })?;

    let mut doc = vec_to_jsonapi_document(releases);
    dedup_document(&mut doc);
    filter_included(
        &mut doc,
        parameters
            .include
            .map_or(Vec::new(), |s| s.split(",").map(|s| s.to_owned()).collect()),
    );
    Ok(Response(doc))
}

pub async fn release(
    State(AppState(db)): State<AppState>,
    Path(id): Path<Uuid>,
    Query(parameters): Query<QueryParameters>,
) -> Result<Response, Error> {
    let tx = db.begin().await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't being database transaction".to_string(),
            e.into(),
        )
    })?;
    let release = entity::ReleaseEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not fetch release".to_string(),
                e.into(),
            )
        })?
        .ok_or(Error(
            StatusCode::NOT_FOUND,
            "Not found".to_string(),
            "Not found".into(),
        ))?;
    let RelatedToReleases(artists, mediums, releases) =
        find_related_to_releases(&tx, vec![release])
            .await
            .map_err(|e| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not fetch entites related to the release".to_string(),
                    e.into(),
                )
            })?;
    let (release, artist_credits, image) = releases.first().unwrap();
    let track = related_to_release(release, artist_credits, image, &artists, &mediums);
    let mut doc = track.to_jsonapi_document();
    dedup_document(&mut doc);
    filter_included(
        &mut doc,
        parameters
            .include
            .map_or(Vec::new(), |s| s.split(",").map(|s| s.to_owned()).collect()),
    );
    Ok(Response(doc))
}
