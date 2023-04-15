use axum::http::StatusCode;
use base::database::get_database;
use chrono::Datelike;
use jsonapi::api::*;
use jsonapi::jsonapi_model;
use jsonapi::model::*;
use sea_orm::{EntityTrait, LoaderTrait, ModelTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::response::{Error, Response};

#[derive(Serialize, Deserialize, Default)]
struct Track {
    // mandatory
    id: Uuid,
    title: String,
    artists: Vec<String>,
    album: String,

    track: u32,
    tracktotal: Option<u32>,
    disc: u32,
    disctotal: Option<u32>,
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,
    bpm: Option<u32>,
    genres: Vec<String>,
    #[serde(rename = "recording-mbid")]
    recording_mbid: Uuid,
    #[serde(rename = "track-mbid")]
    track_mbid: Uuid,
    composers: Vec<String>,
    albumartist: Option<String>,
    comments: Option<String>,
}

jsonapi_model!(Track; "track");

pub async fn tracks() -> Result<Response, Error> {
    let db = get_database().map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not get database connection".to_string(),
            e.into(),
        )
    })?;
    let mut results = vec![];
    let tracks = entity::TrackEntity::find()
        .find_with_related(entity::ArtistCreditEntity)
        .all(db)
        .await
        .map_err(|e| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not fetch tracks".to_string(),
                e.into(),
            )
        })?;
    for (track, artist_credit) in tracks.into_iter() {
        let artists = artist_credit
            .load_one(entity::ArtistEntity, db)
            .await
            .map_err(|e| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not find related Artist".to_string(),
                    e.into(),
                )
            })?;
        // let artist_track_relation = track
        //     .find_related(entity::ArtistTrackRelationEntity)
        //     .all(db)
        //     .await
        //     .map_err(|e| {
        //         Error(
        //             StatusCode::INTERNAL_SERVER_ERROR,
        //             "Could not find related ArtistTrackRelation".to_string(),
        //             e.into(),
        //         )
        //     })?;
        let medium = track
            .find_related(entity::MediumEntity)
            .one(db)
            .await
            .map_err(|e| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not find related Medium".to_string(),
                    e.into(),
                )
            })?
            .ok_or({
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "No medium ralted to the track was found".to_string(),
                    "No medium ralted to the track was found".into(),
                )
            })?;
        let release = medium
            .find_related(entity::ReleaseEntity)
            .one(db)
            .await
            .map_err(|e| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not find related Release".to_string(),
                    e.into(),
                )
            })?
            .ok_or({
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "No release ralted to the medium was found".to_string(),
                    "No release ralted to the medium was found".into(),
                )
            })?;
        let document = Track {
            id: track.id,
            title: track.title,
            artists: artist_credit
                .into_iter()
                .filter_map(|ac| {
                    let artist = artists
                        .iter()
                        .find(|artist| {
                            artist
                                .as_ref()
                                .map(|a| a.id == ac.artist_id)
                                .unwrap_or(false)
                        })
                        .map(|a| a.clone().unwrap()); // at this point the value is _certainly_ Some(..)
                    artist.map(|a| a.name)
                })
                .collect(),
            album: release.title,

            track: track.number,
            // TODO: total in release maybe (?)
            // tracktotal: --
            disc: medium.position + 1,
            // TODO: total in release maybe (?)
            // disctotal: --
            year: release.date.map(|d| d.year()),
            month: release.date.map(|d| d.month()),
            day: release.date.map(|d| d.day()),
            // TODO: bpm somehow?
            genres: track.genres.0,
            recording_mbid: track.recording_id,
            track_mbid: track.id,
            // TODO: weired err
            // composers: vec![],
            // TODO: release artists
            // albumartist: None,
            ..Default::default()
        };
        results.push(document);
    }
    Ok(Response(vec_to_jsonapi_document(results)))
}
