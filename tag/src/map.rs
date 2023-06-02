use crate::TagKey;
use entity::{
    full::{ArtistInfo, FullRelease, FullTrack},
    Artist, ArtistTrackRelationType,
};
use eyre::{eyre, Result};
use std::collections::HashMap;

pub type TagMap = HashMap<TagKey, Vec<String>>;
pub type StringMap = HashMap<String, String>;

fn artist_names(artists: Vec<&Artist>) -> Vec<String> {
    artists.into_iter().map(|a| a.name.to_string()).collect()
}

pub fn tags_from_full_track(full_track: &FullTrack) -> Result<TagMap> {
    let FullTrack { track, .. } = &full_track;
    let mut map = HashMap::new();
    map.insert(TagKey::MusicBrainzTrackID, vec![track.id.to_string()]);
    map.insert(TagKey::TrackTitle, vec![track.title.clone()]);

    map.insert(TagKey::Artists, artist_names(full_track.get_artists()?));
    map.insert(TagKey::Artist, artist_names(full_track.get_artists()?));
    map.insert(
        TagKey::MusicBrainzArtistID,
        full_track
            .get_artists()?
            .into_iter()
            .map(|a| a.id.to_string())
            .collect(),
    );
    map.insert(
        TagKey::ArtistSortOrder,
        full_track
            .get_artists()?
            .into_iter()
            .map(|a| a.sort_name.to_string())
            .collect(),
    );
    map.insert(TagKey::MusicBrainzDiscID, vec![track.medium_id.to_string()]);
    map.insert(TagKey::Duration, vec![track.length.to_string()]);
    map.insert(TagKey::TrackNumber, vec![track.number.to_string()]);
    map.insert(TagKey::Genre, track.genres.0.clone());
    map.insert(
        TagKey::Performer,
        artist_names(full_track.get_related(ArtistTrackRelationType::Performer)?),
    );
    map.insert(
        TagKey::Engineer,
        artist_names(full_track.get_related(ArtistTrackRelationType::Engineer)?),
    );
    map.insert(
        TagKey::Mixer,
        artist_names(full_track.get_related(ArtistTrackRelationType::Mix)?),
    );
    map.insert(
        TagKey::Producer,
        artist_names(full_track.get_related(ArtistTrackRelationType::Producer)?),
    );
    map.insert(
        TagKey::Lyricist,
        artist_names(full_track.get_related(ArtistTrackRelationType::Lyricist)?),
    );
    map.insert(
        TagKey::Writer,
        artist_names(full_track.get_related(ArtistTrackRelationType::Writer)?),
    );
    map.insert(
        TagKey::Composer,
        artist_names(full_track.get_related(ArtistTrackRelationType::Composer)?),
    );
    map.insert(
        TagKey::ComposerSortOrder,
        full_track
            .get_related(ArtistTrackRelationType::Composer)?
            .into_iter()
            .map(|a| a.sort_name.to_string())
            .collect(),
    );
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
    if let Some(rel_year) = &release.year {
        map.insert(TagKey::ReleaseYear, vec![rel_year.to_string()]);
        let month = release
            .month
            .map(|m| m.to_string())
            .map_or(String::new(), |m| "-".to_string() + m.as_str());
        let day = release
            .day
            .map(|m| m.to_string())
            .map_or(String::new(), |d| "-".to_string() + d.as_str());
        let date = rel_year.to_string() + month.as_str() + day.as_str();
        map.insert(TagKey::ReleaseDate, vec![date]);
    }
    if let Some(rel_original_year) = &release.original_year {
        map.insert(
            TagKey::OriginalReleaseYear,
            vec![rel_original_year.to_string()],
        );
        let month = release
            .original_month
            .map(|m| m.to_string())
            .map_or(String::new(), |m| "-".to_string() + m.as_str());
        let day = release
            .original_day
            .map(|m| m.to_string())
            .map_or(String::new(), |d| "-".to_string() + d.as_str());
        let date = rel_original_year.to_string() + month.as_str() + day.as_str();
        map.insert(TagKey::OriginalReleaseDate, vec![date]);
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
            .get_artists()?
            .into_iter()
            .map(|a| a.name.to_string())
            .collect(),
    );
    map.insert(
        TagKey::AlbumArtistSortOrder,
        full_release
            .get_artists()?
            .into_iter()
            .map(|a| a.sort_name.clone())
            .collect(),
    );
    map.insert(
        TagKey::MusicBrainzReleaseArtistID,
        full_release
            .get_artists()?
            .into_iter()
            .map(|a| a.id.to_string())
            .collect(),
    );
    map.insert(TagKey::TotalDiscs, vec![medium.len().to_string()]);
    map.insert(
        TagKey::TotalTracks,
        vec![medium.iter().fold(0, |v, e| v + e.tracks).to_string()],
    );
    Ok(map)
}

pub fn tags_from_artist(artist: &Artist) -> Result<TagMap> {
    let Artist {
        name, sort_name, ..
    } = &artist;
    let mut map = HashMap::new();
    map.insert(TagKey::Artist, vec![name.to_owned()]);
    map.insert(TagKey::ArtistSortOrder, vec![sort_name.to_owned()]);
    Ok(map)
}

pub fn tags_from_combination(full_release: &FullRelease, full_track: &FullTrack) -> Result<TagMap> {
    let mut map = tags_from_full_release(full_release)?;
    map.extend(tags_from_full_track(full_track)?);
    let index = full_release
        .medium
        .iter()
        .position(|m| m.id == full_track.track.medium_id)
        .ok_or(eyre!("track references a missing medium"))?;
    map.insert(TagKey::DiscNumber, vec![(index + 1).to_string()]);
    Ok(map)
}

pub fn strs_from_combination(
    full_release: &FullRelease,
    full_track: &FullTrack,
) -> Result<StringMap> {
    tags_from_combination(full_release, full_track).map(|t| tag_to_string_map(&t))
}

pub fn tag_to_string_map(input: &TagMap) -> StringMap {
    let mut map: StringMap = HashMap::new();
    for (k, v) in input.iter() {
        if let Some(val) = v.first() {
            map.insert(k.to_string(), val.to_string());
        }
    }
    map
}

pub fn sanitize_filename(str: &str) -> String {
    str.replace(['/', '\\'], "-")
}

pub fn sanitize_map(map: StringMap) -> StringMap {
    map.into_iter()
        .map(|(k, v)| {
            let sanitized_value = if v.contains('/') {
                sanitize_filename(&v)
            } else {
                v
            };
            (k, sanitized_value)
        })
        .collect()
}
