use crate::TagKey;
use chrono::Datelike;
use entity::full::{ArtistInfo, FullRelease, FullTrack};
use eyre::Result;
use std::collections::HashMap;

pub type TagMap = HashMap<TagKey, Vec<String>>;
pub type StringMap = HashMap<String, Vec<String>>;

pub fn tags_from_full_track(full_track: &FullTrack) -> Result<TagMap> {
    let FullTrack { track, .. } = &full_track;
    let mut map = HashMap::new();
    map.insert(TagKey::MusicBrainzTrackID, vec![track.id.to_string()]);
    map.insert(TagKey::TrackTitle, vec![track.title.clone()]);

    // artists
    let artist_names: Vec<String> = full_track
        .artists()?
        .into_iter()
        .map(|a| a.name.to_string())
        .collect();
    map.insert(TagKey::Artists, artist_names.clone());
    map.insert(TagKey::Artist, artist_names);
    map.insert(
        TagKey::MusicBrainzArtistID,
        full_track
            .artists()?
            .into_iter()
            .map(|a| a.id.to_string())
            .collect(),
    );
    map.insert(
        TagKey::ArtistSortOrder,
        full_track
            .artists()?
            .into_iter()
            .map(|a| a.id.to_string())
            .collect(),
    );
    map.insert(TagKey::Duration, vec![track.length.to_string()]);
    // map.insert(TagKey::DiscNumber, vec![track.disc.to_string()]);
    // if let Some(disc_mbid) = track.disc_mbid {
    //     map.insert(TagKey::MusicBrainzDiscID, vec![disc_mbid]);
    // }
    map.insert(TagKey::TrackNumber, vec![track.number.to_string()]);
    map.insert(TagKey::Genre, track.genres.0.clone());
    // map.insert(TagKey::Performer, track.performers);
    // map.insert(TagKey::Engineer, track.engigneers);
    // map.insert(TagKey::Mixer, track.mixers);
    // map.insert(TagKey::Producer, track.producers);
    // map.insert(TagKey::Lyricist, track.lyricists);
    // map.insert(TagKey::Writer, track.writers);
    // map.insert(TagKey::Composer, track.composers);
    // map.insert(TagKey::ComposerSortOrder, track.composers);
    Ok(map)
}

pub fn tags_from_full_release(full_release: &FullRelease) -> Result<TagMap> {
    let FullRelease {
        release, medium, ..
    } = &full_release;
    let mut map = HashMap::new();
    map.insert(TagKey::MusicBrainzReleaseID, vec![release.id.to_string()]);
    if let Some(rel_group_id) = &release.release_group_id {
        map.insert(
            TagKey::MusicBrainzReleaseGroupID,
            vec![rel_group_id.to_string()],
        );
    }
    map.insert(
        TagKey::ASIN,
        release.asin.as_ref().map_or(vec![], |a| vec![a.clone()]),
    );
    if let Some(rel_country) = &release.country {
        map.insert(TagKey::ReleaseCountry, vec![rel_country.to_string()]);
    }
    if let Some(rel_label) = &release.label {
        map.insert(TagKey::RecordLabel, vec![rel_label.to_string()]);
    }
    if let Some(rel_catno) = &release.catalog_no {
        map.insert(TagKey::CatalogNumber, vec![rel_catno.to_string()]);
    }
    if let Some(rel_status) = &release.status {
        map.insert(TagKey::ReleaseStatus, vec![rel_status.to_string()]);
    }
    if let Some(rel_type) = &release.release_type {
        map.insert(TagKey::ReleaseType, vec![rel_type.to_string()]);
    }
    if let Some(rel_date) = &release.date {
        map.insert(TagKey::ReleaseDate, vec![rel_date.to_string()]);
        map.insert(TagKey::ReleaseYear, vec![rel_date.year().to_string()]);
    }
    if let Some(rel_original_date) = &release.original_date {
        map.insert(
            TagKey::OriginalReleaseDate,
            vec![rel_original_date.to_string()],
        );
        map.insert(
            TagKey::OriginalReleaseYear,
            vec![rel_original_date.year().to_string()],
        );
    }
    if let Some(rel_script) = &release.script {
        map.insert(TagKey::Script, vec![rel_script.to_string()]);
    }
    if let Some(media_format) = &medium.first().and_then(|m| m.format.as_ref()) {
        map.insert(TagKey::Media, vec![media_format.to_string()]);
    }
    map.insert(TagKey::Album, vec![release.title.clone()]);
    map.insert(TagKey::AlbumSortOrder, vec![release.title.clone()]);
    map.insert(
        TagKey::AlbumArtist,
        full_release
            .artists()?
            .into_iter()
            .map(|a| a.name.to_string())
            .collect(),
    );
    map.insert(
        TagKey::AlbumArtistSortOrder,
        full_release
            .artists()?
            .into_iter()
            .map(|a| a.sort_name.clone())
            .collect(),
    );
    map.insert(
        TagKey::MusicBrainzReleaseArtistID,
        full_release
            .artists()?
            .into_iter()
            .map(|a| a.id.to_string())
            .collect(),
    );
    map.insert(TagKey::TotalDiscs, vec![medium.len().to_string()]);
    map.insert(
        TagKey::TotalTracks,
        vec![medium.into_iter().fold(0, |v, e| v + e.tracks).to_string()],
    );
    Ok(map)
}
pub fn tag_to_string_map(input: TagMap) -> StringMap {
    let mut map: StringMap = HashMap::new();
    for (k, v) in input.into_iter() {
        map.insert(k.to_string(), v);
    }
    map
}
