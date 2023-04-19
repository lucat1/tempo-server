use std::collections::VecDeque;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use chrono::Datelike;
use entity::RelationType;
use eyre::{eyre, Result};
use jsonapi::model::*;
use sea_orm::{ConnectionTrait, DbConn, EntityTrait, LoaderTrait, ModelTrait, TransactionTrait};
use tower::util::ServiceExt;
use uuid::Uuid;

use super::documents::Release;
use crate::response::{Error, Response};
use web::{AppState, QueryParameters};

#[derive(Debug)]
struct RelatedToReleases(
    pub HashMap<Uuid, entity::Artist>,
    pub HashMap<Uuid, entity::Medium>,
    pub  Vec<(
        entity::Release,
        Vec<entity::ArtistCredit>,
        Vec<entity::Track>,
    )>,
);

async fn find_related_to_release(
    db: &C,
    releases: Vec<entity::Release>,
) where C: ConnectInfo -> Result<RelatedToReleases> {
    !unimplemented!()
}

pub fn related_to_release(r: RelatedToReleases) -> Release {
    let (release, artist_credits, image, artists, mediums) = r;
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
    let r = find_related_to_release(&tx, releases).await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not fetch entites related to the tracks".to_string(),
            e.into(),
        )
    })?;
    let tracks = related_to_release(&r).map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not aggregate relation data".to_string(),
            e.into(),
        )
    })?;

    let mut doc = vec_to_jsonapi_document(tracks);
    // dedup_document(&mut doc);
    // filter_included(
    //     &mut doc,
    //     parameters
    //         .include
    //         .map_or(Vec::new(), |s| s.split(",").map(|s| s.to_owned()).collect()),
    // );
    Ok(Response(doc))
}
