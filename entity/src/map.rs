use super::{FullRelease, FullTrack, TagKey};
use chrono::Datelike;
use eyre::{Report, Result};
use std::collections::HashMap;

pub type KeyMap = HashMap<TagKey, Vec<String>>;

impl TryFrom<FullTrack> for KeyMap {
    type Error = Report;
    fn try_from(full_track: FullTrack) -> Result<Self, Self::Error> {
        let FullTrack(track, _, _, _) = &full_track;
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
}

impl TryFrom<FullRelease> for HashMap<TagKey, Vec<String>> {
    type Error = Report;
    fn try_from(full_release: FullRelease) -> Result<Self, Self::Error> {
        let FullRelease(release, mediums, _, _) = &full_release;
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
        if let Some(media_format) = &mediums.first().and_then(|m| m.format.as_ref()) {
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
                .map(|a| a.sort_name)
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
        map.insert(TagKey::TotalDiscs, vec![mediums.len().to_string()]);
        map.insert(
            TagKey::TotalTracks,
            vec![mediums.into_iter().fold(0, |v, e| v + e.tracks).to_string()],
        );
        Ok(map)
    }
}

impl TryFrom<FullTrack> for HashMap<String, Vec<String>> {
    type Error = Report;
    fn try_from(track: FullTrack) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();
        let tag_map: HashMap<TagKey, Vec<String>> = track.try_into()?;
        for (k, v) in tag_map.into_iter() {
            map.insert(k.to_string(), v);
        }
        Ok(map)
    }
}

impl TryFrom<FullRelease> for HashMap<String, Vec<String>> {
    type Error = Report;
    fn try_from(release: FullRelease) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();
        let tag_map: HashMap<TagKey, Vec<String>> = release.try_into()?;
        for (k, v) in tag_map.into_iter() {
            map.insert(k.to_string(), v);
        }
        Ok(map)
    }
}
