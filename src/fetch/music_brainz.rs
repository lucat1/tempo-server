use eyre::Report;
use serde_derive::{Deserialize, Serialize};
use setting::get_settings;
use std::cmp::Ordering;
use std::sync::Arc;
use uuid::Uuid;

use crate::util::maybe_date;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    #[serde(rename = "label-info")]
    #[serde(default)]
    pub label_info: Vec<LabelInfo>,
    pub status: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<ReleaseGroup>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    pub asin: Option<String>,
    pub date: Option<String>,
    pub id: Uuid,
    pub media: Vec<Medium>,
    pub country: Option<String>,
    #[serde(rename = "text-representation")]
    pub text_representation: Option<TextRepresentation>,
    pub title: String,
    #[serde(rename = "track-count")]
    pub track_count: Option<usize>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGroup {
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: String,
    pub id: Uuid,
    pub disambiguation: Option<String>,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub joinphrase: Option<String>,
    pub artist: Artist,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    #[serde(rename = "type-id")]
    pub type_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub disambiguation: Option<String>,
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Vec<String>,
    pub id: Uuid,
    pub disambiguation: Option<String>,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Medium {
    pub id: Option<Uuid>,
    pub position: Option<u64>,
    #[serde(rename = "track-offset")]
    pub track_offset: Option<u64>,
    #[serde(rename = "track-count")]
    pub track_count: u64,
    pub tracks: Option<Vec<Track>>,
    pub format: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: Uuid,
    pub recording: Recording,
    pub number: String,
    pub position: u64,
    pub length: Option<u64>,
    pub title: String,

    pub medium: Option<Arc<Medium>>,
    pub release: Option<Arc<Release>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub relations: Vec<Relation>,
    pub disambiguation: String,
    pub id: Uuid,
    pub length: Option<u64>,
    pub video: bool,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: Option<String>,
    pub genres: Option<Vec<Genre>>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "type")]
    pub type_field: String,
    pub artist: Option<Artist>,
    pub work: Option<Work>,
    pub attributes: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    pub relations: Option<Vec<Relation>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genre {
    pub id: Uuid,
    pub count: u64,
    pub disambiguation: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSearch {
    pub created: String,
    pub count: i64,
    pub offset: i64,
    pub releases: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRepresentation {
    pub language: Option<String>,
    pub script: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
    pub label: Option<Label>,
}

impl From<Artist> for entity::Artist {
    fn from(artist: Artist) -> entity::Artist {
        entity::Artist {
            id: artist.id,
            name: artist.name,
            sort_name: artist.sort_name,
        }
    }
}

impl From<ArtistCredit> for entity::ArtistCredit {
    fn from(artist: ArtistCredit) -> Self {
        entity::ArtistCredit {
            id: 0,
            join_phrase: artist.joinphrase,
            artist_id: artist.artist.id,
        }
    }
}

impl From<Track> for entity::FullTrack {
    fn from(track: Track) -> Self {
        let mut sorted_genres = track.recording.genres.unwrap_or_default();
        sorted_genres.sort_by(|a, b| a.count.partial_cmp(&b.count).unwrap_or(Ordering::Equal));
        let mut other_relations = track
            .recording
            .relations
            .iter()
            .filter_map(|rel| {
                if <String as Into<entity::RelationType>>::into(rel.type_field.clone())
                    == entity::RelationType::Performance
                {
                    rel.work.clone()
                } else {
                    None
                }
            })
            .filter_map(|work| work.relations)
            .flatten()
            .collect::<Vec<_>>();
        let mut all_relations = track.recording.relations.clone();
        all_relations.append(&mut other_relations);

        let mut artists: Vec<entity::Artist> = track
            .recording
            .artist_credit
            .as_ref()
            .map_or(vec![], |acs| {
                acs.into_iter().map(|ac| ac.artist.clone().into()).collect()
            });
        // Append artists for all other relations
        artists.append(
            &mut all_relations
                .iter()
                .filter_map(|r| r.artist.as_ref())
                .map(|a| a.clone().into())
                .collect(),
        );
        entity::FullTrack(
            entity::Track {
                id: track.id,
                title: track.title,
                length: track.length.or(track.recording.length).unwrap_or_default(),
                number: track.position,
                genres: entity::Genres(
                    sorted_genres
                        .into_iter()
                        .map(|g| g.name)
                        .collect::<Vec<_>>(),
                ),
                format: None,
                path: None,
            },
            track
                .recording
                .artist_credit
                .unwrap_or_default()
                .into_iter()
                .map(|ac| ac.into())
                .collect(),
            all_relations
                .iter()
                .filter_map(|r| {
                    r.artist.as_ref().map(|a| entity::ArtistTrackRelation {
                        artist_id: a.id,
                        track_id: track.id,
                        relation_type: r.type_field.clone().into(),
                        relation_value: r.type_field.clone(),
                    })
                })
                .collect(),
            artists,
        )
    }
}

impl TryFrom<Release> for entity::FullRelease {
    type Error = Report;
    fn try_from(release: Release) -> Result<Self, Self::Error> {
        let original_date = maybe_date(
            release
                .release_group
                .as_ref()
                .and_then(|r| r.first_release_date.clone()),
        );
        let label = release.label_info.first();
        Ok(entity::FullRelease(
            entity::Release {
                id: release.id,
                title: release.title,
                release_group_id: release.release_group.as_ref().map(|r| r.id),
                release_type: release
                    .release_group
                    .as_ref()
                    .and_then(|r| r.primary_type.as_ref())
                    .map(|s| s.to_lowercase()),
                asin: release.asin,
                country: release.country,
                label: label
                    .as_ref()
                    .and_then(|li| li.label.as_ref())
                    .map(|l| l.name.to_string()),
                catalog_no: label.as_ref().and_then(|l| l.catalog_number.clone()),
                status: release.status,
                date: if get_settings()?.tagging.use_original_date {
                    original_date
                } else {
                    maybe_date(release.date)
                },
                original_date,
                script: release.text_representation.and_then(|t| t.script),
            },
            release
                .media
                .iter()
                .map(|m| entity::Medium {
                    id: m.id.unwrap_or_else(|| Uuid::new_v4()),
                    position: m.position.unwrap_or_default(),
                    tracks: m.track_count,
                    track_offset: m.track_offset.unwrap_or_default(),
                    format: m.format.clone(),
                })
                .collect(),
            release
                .artist_credit
                .iter()
                .map(|ac| ac.clone().into())
                .collect(),
            release
                .artist_credit
                .into_iter()
                .map(|a| a.artist.into())
                .collect(),
        ))
    }
}
