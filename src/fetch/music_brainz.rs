use sea_orm::entity::ActiveValue;
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
    pub id: Uuid,
    pub position: u64,
    pub track_offset: u64,
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
    pub attributes: Vec<String>,
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

// impl From<Artist> for entity::ArtistActive {
//     fn from(artist: Artist) -> Self {
//         entity::ArtistActive {
//             id: ActiveValue::Set(artist.id),
//             name: ActiveValue::Set(artist.name),
//             sort_name: ActiveValue::Set(artist.sort_name),
//         }
//     }
// }
//
// impl From<ArtistCredit> for entity::ArtistCreditActive {
//     fn from(artist: ArtistCredit) -> Self {
//         entity::ArtistCreditActive {
//             join_phrase: ActiveValue::Set(artist.joinphrase),
//             artist_id: ActiveValue::Set(artist.artist.id),
//             ..Default::default()
//         }
//     }
// }

impl From<Track> for entity::FullTrackActive {
    fn from(track: Track) -> Self {
        let mut sorted_genres = track.recording.genres.unwrap_or_default();
        sorted_genres.sort_by(|a, b| a.count.partial_cmp(&b.count).unwrap_or(Ordering::Equal));
        let mut other_relations = track
            .recording
            .relations
            .iter()
            .filter_map(|rel| {
                if rel.type_field.clone().into() == entity::RelationType::Performance {
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

        let mut artists: Vec<entity::ArtistActive> = track.recording.artist_credit.map_or(
            |acs| acs.into_iter().map(|ac| ac.artist.into()).collect(),
            vec![],
        );
        // Append artists for all other relations
        artists.append(all_relations.iter().map(|r| r.artist.into()).collect());
        (
            entity::TrackActive {
                id: ActiveValue::Set(track.id),
                title: track.title,
                length: track.length.or(track.recording.length).into(),
                number: ActiveValue::Set(track.position),
                genres: sorted_genres
                    .into_iter()
                    .map(|g| g.name)
                    .collect::<Vec<_>>(),
                format: ActiveValue::NotSet,
                path: ActiveValue::NotSet,
            },
            track.recording.artist_credit.into(),
            all_relations
                .iter()
                .map(|r| entity::ArtistTrackRelationActive {
                    artist_id: r
                        .artist
                        .id
                        .map_or(|a| ActiveValue::Set(a.id), ActiveValue::NotSet),
                    track_id: ActiveValue::Set(track.id),
                    relation_type: r.type_field.into(),
                    relation_value: r.type_field,
                })
                .collect(),
            artists,
        )
    }
}

impl From<Release> for entity::FullReleaseActive {
    fn from(release: Release) -> Self {
        let original_date = maybe_date(
            release
                .release_group
                .as_ref()
                .and_then(|r| r.first_release_date.clone()),
        );
        let label = release.label_info.first();
        (
            entity::ReleaseActive {
                id: ActiveValue::Set(release.id),
                title: ActiveValue::Set(release.title),
                release_group_id: release
                    .release_group
                    .as_ref()
                    .map_or(|r| ActiveValue::Set(r.id.clone()), ActiveValue::NotSet),
                release_type: release
                    .release_group
                    .as_ref()
                    .and_then(|r| r.primary_type.as_ref())
                    .map_or(
                        |pt| ActiveValue::Set(pt.to_lowercase()),
                        ActiveValue::NotSet,
                    ),
                asin: ActiveValue::Set(release.asin),
                country: release
                    .country
                    .map_or(|c| ActiveValue::Set(c), ActiveValue::NotSet),
                label: label
                    .as_ref()
                    .and_then(|li| li.label.as_ref())
                    .map_or(|l| ActiveValue::Set(l.name.clone()), ActiveValue::NotSet),
                catalog_no: label.as_ref().map_or(
                    |l| ActiveValue::Set(l.catalog_number.clone()),
                    ActiveValue::NotSet,
                ),
                status: release
                    .status
                    .map_or(|v| ActiveValue::Set(v), ActiveValue::NotSet),
                date: if get_settings()?.tagging.use_original_date {
                    original_date
                } else {
                    maybe_date(release.date)
                }
                .map_or(|v| ActiveValue::Set(v), ActiveValue::NotSet),
                original_date: original_date.map_or(|v| ActiveValue::Set(v), ActiveValue::NotSet),
                script: release.text_representation.and_then(|t| t.script),
            },
            release
                .media
                .iter()
                .map(|m| entity::MediumActive {
                    id: ActiveValue::Set(m.id),
                    position: ActiveValue::Set(m.position),
                    track_offset: ActiveValue::Set(m.track_offset),
                    format: m
                        .format
                        .map_or(|f| ActiveValue::Set(f), ActiveValue::NotSet),
                })
                .collect(),
            release
                .artist_credit
                .into_iter()
                .map(|a| a.into())
                .collect(),
            release.artist_credit.into_iter().map(|a| a).collect(),
        )
    }
}
