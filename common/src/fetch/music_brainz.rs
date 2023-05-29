use entity::full::FullRelease;
use eyre::Result;
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use uuid::Uuid;

use base::setting::Library;
use base::util::{dedup, maybe_date};

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
    pub media: Option<Vec<Medium>>,
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
    pub position: Option<i32>,
    #[serde(rename = "track-offset")]
    pub track_offset: Option<i32>,
    #[serde(rename = "track-count")]
    pub track_count: i32,
    pub tracks: Option<Vec<Track>>,
    pub format: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: Uuid,
    pub recording: Recording,
    pub number: String,
    pub position: i32,
    pub length: Option<i32>,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub relations: Vec<Relation>,
    pub disambiguation: String,
    pub id: Uuid,
    pub length: Option<i32>,
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
    pub count: u32,
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

            description: None,
        }
    }
}

fn artist_credit_id(ac: &ArtistCredit) -> String {
    return ac.artist.id.to_string() + "-" + ac.joinphrase.as_ref().map_or("", |s| s.as_str());
}

impl From<ArtistCredit> for entity::ArtistCredit {
    fn from(artist_credit: ArtistCredit) -> Self {
        entity::ArtistCredit {
            id: artist_credit_id(&artist_credit),
            join_phrase: artist_credit.joinphrase,
            artist_id: artist_credit.artist.id,
        }
    }
}

pub struct TrackWithMediumId(pub Track, pub Uuid);

impl From<TrackWithMediumId> for entity::full::FullTrack {
    fn from(TrackWithMediumId(track, medium_id): TrackWithMediumId) -> Self {
        let mut sorted_genres = track.recording.genres.unwrap_or_default();
        sorted_genres.sort_by(|a, b| a.count.partial_cmp(&b.count).unwrap_or(Ordering::Equal));
        let mut other_relations = track
            .recording
            .relations
            .iter()
            .filter_map(|rel| {
                if <String as Into<entity::ArtistTrackRelationType>>::into(rel.type_field.clone())
                    == entity::ArtistTrackRelationType::Performance
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
                acs.iter().map(|ac| ac.artist.clone().into()).collect()
            });
        // Append artists for all other relations
        artists.append(
            &mut all_relations
                .iter()
                .filter_map(|r| r.artist.as_ref())
                .map(|a| a.clone().into())
                .collect(),
        );
        entity::full::FullTrack {
            track: entity::Track {
                id: track.id,
                medium_id,
                title: track.title,
                length: track.length.or(track.recording.length).unwrap_or_default(),
                number: track.position,
                genres: entity::Genres(
                    sorted_genres
                        .into_iter()
                        .map(|g| g.name)
                        .collect::<Vec<_>>(),
                ),
                recording_id: track.recording.id,
                format: None,
                path: None,
            },
            artist_credit_track: track
                .recording
                .artist_credit
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|ac| entity::ArtistCreditTrack {
                    artist_credit_id: artist_credit_id(&ac),
                    track_id: track.id,
                })
                .collect(),
            artist_credit: track
                .recording
                .artist_credit
                .unwrap_or_default()
                .into_iter()
                .map(|ac| ac.into())
                .collect(),
            artist_track_relation: all_relations
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
            artist: artists,
        }
    }
}

impl Release {
    pub fn into_full_release(self, library: &Library) -> Result<FullRelease> {
        let original_date = maybe_date(
            self.release_group
                .as_ref()
                .and_then(|r| r.first_release_date.clone()),
        );
        let label = self.label_info.first();
        let genres = self
            .media
            .clone()
            .unwrap_or_default()
            .iter()
            .filter_map(|m| m.tracks.as_ref())
            .flatten()
            .filter_map(|t| t.recording.genres.as_ref())
            .flatten()
            .map(|g| g.name.to_owned())
            .collect::<Vec<_>>();

        Ok(entity::full::FullRelease {
            release: entity::Release {
                id: self.id,
                title: self.title,
                release_group_id: self.release_group.as_ref().map(|r| r.id),
                release_type: self
                    .release_group
                    .as_ref()
                    .and_then(|r| r.primary_type.as_ref())
                    .map(|s| s.to_lowercase()),
                genres: entity::Genres(dedup(genres)),
                asin: self.asin,
                country: self.country,
                label: label
                    .as_ref()
                    .and_then(|li| li.label.as_ref())
                    .map(|l| l.name.to_string()),
                catalog_no: label.as_ref().and_then(|l| l.catalog_number.clone()),
                status: self.status,
                date: if library.tagging.use_original_date {
                    original_date
                } else {
                    maybe_date(self.date)
                },
                original_date,
                script: self.text_representation.and_then(|t| t.script),
                path: None,
            },
            medium: self
                .media
                .unwrap_or_default()
                .iter()
                .map(|m| entity::Medium {
                    id: m.id.unwrap_or_else(Uuid::new_v4),
                    release_id: self.id,
                    position: m.position.unwrap_or_default(),
                    tracks: m.track_count,
                    track_offset: m.track_offset.unwrap_or_default(),
                    format: m.format.clone(),
                })
                .collect(),
            artist_credit_release: self
                .artist_credit
                .iter()
                .map(|ac| entity::ArtistCreditRelease {
                    artist_credit_id: artist_credit_id(ac),
                    release_id: self.id,
                })
                .collect(),
            artist_credit: self
                .artist_credit
                .iter()
                .map(|ac| ac.clone().into())
                .collect(),
            artist: self
                .artist_credit
                .into_iter()
                .map(|a| a.artist.into())
                .collect(),
        })
    }
}
