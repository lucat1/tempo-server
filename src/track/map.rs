use crate::models::{Artists, Track};

use super::key::TagKey;
use crate::SETTINGS;
use chrono::Datelike;
use eyre::{eyre, Report, Result};
use std::collections::HashMap;

impl TryFrom<Track> for HashMap<TagKey, Vec<String>> {
    type Error = Report;
    fn try_from(track: Track) -> Result<Self, Self::Error> {
        let settings = SETTINGS.get().ok_or(eyre!("Could not get settings"))?;
        let mut map = HashMap::new();
        if let Some(id) = track.mbid {
            map.insert(TagKey::MusicBrainzTrackID, vec![id]);
        }
        if let Some(release) = track.release {
            if let Some(rel_id) = &release.mbid {
                map.insert(TagKey::MusicBrainzReleaseID, vec![rel_id.clone()]);
            }
            if let Some(rel_group_id) = &release.release_group_mbid {
                map.insert(
                    TagKey::MusicBrainzReleaseGroupID,
                    vec![rel_group_id.clone()],
                );
            }
            if let Some(rel_asin) = &release.asin {
                map.insert(TagKey::ASIN, vec![rel_asin.to_string()]);
            }
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
            if let Some(rel_media) = &release.media {
                map.insert(TagKey::Media, vec![rel_media.to_string()]);
            }
            map.insert(TagKey::Album, vec![release.title.clone()]);
            map.insert(TagKey::AlbumSortOrder, vec![release.title.clone()]);
            map.insert(TagKey::AlbumArtist, release.artists.names());
            map.insert(TagKey::AlbumArtistSortOrder, release.artists.sort_order());
            map.insert(TagKey::MusicBrainzReleaseArtistID, release.artists.ids());
            if let Some(discs) = release.discs {
                map.insert(TagKey::TotalDiscs, vec![discs.to_string()]);
            }
            if let Some(tracks) = release.tracks {
                map.insert(TagKey::TotalTracks, vec![tracks.to_string()]);
            }
        }
        map.insert(TagKey::TrackTitle, vec![track.title]);

        // artists
        map.insert(TagKey::Artists, track.artists.names());
        map.insert(TagKey::Artist, track.artists.names());
        map.insert(TagKey::MusicBrainzArtistID, track.artists.ids());
        map.insert(TagKey::ArtistSortOrder, track.artists.sort_order());
        if let Some(len) = track.length {
            map.insert(TagKey::Duration, vec![len.as_secs().to_string()]);
        }
        if let Some(disc) = track.disc {
            map.insert(TagKey::DiscNumber, vec![disc.to_string()]);
        }
        if let Some(disc_mbid) = track.disc_mbid {
            map.insert(TagKey::MusicBrainzDiscID, vec![disc_mbid.to_string()]);
        }
        if let Some(number) = track.number {
            map.insert(TagKey::TrackNumber, vec![number.to_string()]);
        }
        map.insert(
            TagKey::Genre,
            match settings.tagging.genre_limit {
                None => track.genres,
                Some(l) => track.genres.into_iter().take(l).collect(),
            },
        );
        map.insert(TagKey::Performer, track.performers.instruments());
        map.insert(TagKey::Engineer, track.engigneers.names());
        map.insert(TagKey::Mixer, track.mixers.names());
        map.insert(TagKey::Producer, track.producers.names());
        map.insert(TagKey::Lyricist, track.lyricists.names());
        map.insert(TagKey::Writer, track.writers.names());
        map.insert(TagKey::Composer, track.composers.names());
        map.insert(TagKey::ComposerSortOrder, track.composers.sort_order());
        Ok(map)
    }
}

impl TryFrom<Track> for HashMap<String, Vec<String>> {
    type Error = Report;
    fn try_from(track: Track) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();
        let tag_map: HashMap<TagKey, Vec<String>> = track.try_into()?;
        for (k, v) in tag_map.into_iter() {
            map.insert(k.to_string(), v);
        }
        Ok(map)
    }
}
