use std::collections::VecDeque;

use axum::http::StatusCode;
use base::database::get_database;
use chrono::Datelike;
use entity::RelationType;
use eyre::{eyre, Result};
use jsonapi::api::*;
use jsonapi::jsonapi_model;
use jsonapi::model::*;
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, ModelTrait, TransactionTrait};
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
    tracktotal: u32,
    disc: u32,
    disctotal: u32,
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,
    bpm: Option<u32>,
    genres: Vec<String>,
    #[serde(rename = "recording-mbid")]
    recording_mbid: Uuid,
    #[serde(rename = "track-mbid")]
    track_mbid: Uuid,
    albumartists: Vec<String>,
    comments: Option<String>,

    mimetype: String,
    duration: Option<f32>,
    framerate: Option<u32>,
    framecount: Option<u32>,
    channels: Option<u32>,
    bitrate: Option<u32>,
    bitdepth: Option<u32>,
    size: Option<u32>,

    engigneers: Vec<String>,
    instrumentalists: Vec<String>,
    performers: Vec<String>,
    mixers: Vec<String>,
    producers: Vec<String>,
    vocalists: Vec<String>,
    lyricists: Vec<String>,
    writers: Vec<String>,
    composers: Vec<String>,
    others: Vec<String>,
}

jsonapi_model!(Track; "track");

#[derive(Debug)]
struct RelatedToTracks(
    pub HashMap<Uuid, entity::Artist>,
    pub HashMap<Uuid, entity::Medium>,
    pub HashMap<Uuid, (entity::Release, Vec<entity::ArtistCredit>)>,
    pub  Vec<(
        entity::Track,
        Vec<entity::ArtistCredit>,
        Vec<entity::ArtistTrackRelation>,
    )>,
);

async fn find_related_to_tracks<'a, C>(
    db: &'a C,
    src_tracks: Vec<entity::Track>,
) -> Result<RelatedToTracks>
where
    C: ConnectionTrait,
{
    let mut artists = HashMap::new();
    let mut mediums = HashMap::new();
    let mut releases = HashMap::new();
    let mut tracks = Vec::new();

    let _artist_credits = src_tracks
        .load_many_to_many(
            entity::ArtistCreditEntity,
            entity::ArtistCreditTrackEntity,
            db,
        )
        .await?;
    let artsts = _artist_credits
        .clone()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .load_one(entity::ArtistEntity, db)
        .await?
        .into_iter()
        .flatten();
    for artist in artsts {
        artists.insert(artist.id, artist);
    }
    let mut track_artist_credits: VecDeque<_> = _artist_credits.into();
    let _artist_track_relations = src_tracks
        .load_many(entity::ArtistTrackRelationEntity, db)
        .await?;
    let artsts = _artist_track_relations
        .clone()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .load_one(entity::ArtistEntity, db)
        .await?
        .into_iter()
        .flatten();
    for artist in artsts {
        artists.insert(artist.id, artist);
    }
    let mut artist_track_relations: VecDeque<_> = _artist_track_relations.into();

    for track in src_tracks.into_iter() {
        let release = track
            .find_linked(entity::TrackToRelease)
            .one(db)
            .await?
            .ok_or(eyre!("Track {} doesn't belong to any release", track.id))?;
        let acs = track
            .find_related(entity::ArtistCreditEntity)
            .find_also_related(entity::ArtistEntity)
            .all(db)
            .await?;
        let mut release_artist_credits = vec![];
        for (ac, ar) in acs.into_iter() {
            release_artist_credits.push(ac);
            let artist = ar.ok_or(eyre!("Broken artist_credit relationship"))?;
            artists.insert(artist.id, artist);
        }
        let medms = release.find_related(entity::MediumEntity).all(db).await?;
        releases.insert(release.id, (release, release_artist_credits));
        for medium in medms.into_iter() {
            mediums.insert(medium.id, medium);
        }
        tracks.push((
            track,
            track_artist_credits
                .pop_front()
                .ok_or(eyre!("Missing artist credits relations"))?,
            artist_track_relations
                .pop_front()
                .ok_or(eyre!("Missing artist track relations"))?,
        ))
    }

    Ok(RelatedToTracks(artists, mediums, releases, tracks))
}

fn related_to_tracks(r: RelatedToTracks) -> Result<Vec<Track>> {
    let RelatedToTracks(artists, mediums, releases, tracks) = r;
    let mut results = vec![];
    for (track, track_artist_credits, artist_relations) in tracks.into_iter() {
        let medium = mediums
            .get(&track.medium_id)
            .ok_or(eyre!("Track {} doesn't belong to any medium", track.id))?;
        let (release, release_artist_credits) = releases
            .get(&medium.release_id)
            .ok_or(eyre!("Medium {} doesn't belong to any release", medium.id))?;

        let get_artists_for_relation_type = |rel_type: RelationType| -> Vec<String> {
            artist_relations
                .iter()
                .filter(|ar| ar.relation_type == rel_type)
                .filter_map(|ar| artists.get(&ar.artist_id))
                .map(|a| a.name.clone())
                .collect()
        };

        results.push(Track {
            id: track.id,
            title: track.title,
            artists: track_artist_credits
                .into_iter()
                .filter_map(|ac| artists.get(&ac.artist_id).map(|a| a.name.clone()))
                .collect(),
            album: release.title.clone(),

            track: track.number,
            tracktotal: mediums
                .iter()
                .filter(|(_, med)| med.release_id == release.id)
                .fold(0, |sum, (_, med)| sum + med.tracks),
            disc: medium.position + 1,
            disctotal: mediums
                .iter()
                .filter(|(_, med)| med.release_id == release.id)
                .count() as u32,
            year: release.date.map(|d| d.year()),
            month: release.date.map(|d| d.month()),
            day: release.date.map(|d| d.day()),
            // TODO: bpm somehow?
            genres: track.genres.0,
            recording_mbid: track.recording_id,
            track_mbid: track.id,
            albumartists: release_artist_credits
                .into_iter()
                .filter_map(|ac| artists.get(&ac.artist_id).map(|a| a.name.clone()))
                .collect(),

            mimetype: track
                .format
                .ok_or(eyre!("Track doesn't have a format specified"))?
                .mime(),

            engigneers: get_artists_for_relation_type(RelationType::Engineer),
            instrumentalists: get_artists_for_relation_type(RelationType::Instrument),
            performers: get_artists_for_relation_type(RelationType::Performer),
            mixers: get_artists_for_relation_type(RelationType::Mix),
            producers: get_artists_for_relation_type(RelationType::Producer),
            vocalists: get_artists_for_relation_type(RelationType::Vocal),
            lyricists: get_artists_for_relation_type(RelationType::Lyricist),
            writers: get_artists_for_relation_type(RelationType::Writer),
            composers: get_artists_for_relation_type(RelationType::Composer),
            // TODO: how to call "RelationType::Performance"
            others: get_artists_for_relation_type(RelationType::Other),
            ..Default::default()
        });
    }
    Ok(results)
}

pub async fn tracks() -> Result<Response, Error> {
    let db = get_database().map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not get database connection".to_string(),
            e.into(),
        )
    })?;
    let tx = db.begin().await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't being database transaction".to_string(),
            e.into(),
        )
    })?;
    let tracks = entity::TrackEntity::find().all(&tx).await.map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not fetch tracks".to_string(),
            e.into(),
        )
    })?;
    let r = find_related_to_tracks(&tx, tracks).await.map_err(|e| {
        println!("{:?}", e);
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not fetch entites related to the tracks".to_string(),
            e.into(),
        )
    })?;
    let tracks = related_to_tracks(r).map_err(|e| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not aggregate relation data".to_string(),
            e.into(),
        )
    })?;

    Ok(Response(vec_to_jsonapi_document(tracks)))
}
