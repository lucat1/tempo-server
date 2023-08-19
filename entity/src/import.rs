use base::setting::ArtProvider;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap, hash::Hash};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InternalTrack {
    pub title: String,
    pub artists: Vec<String>,
    pub length: Option<i32>,
    pub disc: Option<i32>,
    pub number: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct InternalRelease {
    pub title: String,
    pub artists: Vec<String>,
    pub media: Option<String>,
    pub discs: Option<i32>,
    pub tracks: i32,
    pub country: Option<String>,
    pub label: Option<String>,
    pub release_type: Option<String>,
    pub year: Option<i32>,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub original_year: Option<i32>,
    pub original_month: Option<u8>,
    pub original_day: Option<u8>,
}

impl From<crate::full::FullRelease> for InternalRelease {
    fn from(full_release: crate::full::FullRelease) -> Self {
        let crate::full::FullRelease {
            release,
            medium,
            artist,
            ..
        } = full_release;
        InternalRelease {
            title: release.title,
            artists: artist.into_iter().map(|a| a.name).collect(),
            discs: Some(medium.len() as i32),
            media: medium.first().as_ref().and_then(|m| m.format.clone()),
            tracks: medium.iter().fold(0, |acc, m| acc + m.tracks),
            country: release.country,
            label: release.label,
            release_type: release.release_type,
            year: release.year,
            month: release.month.map(|m| m as u8),
            day: release.day.map(|d| d as u8),
            original_year: release.original_year,
            original_month: release.original_month.map(|m| m as u8),
            original_day: release.original_day.map(|d| d as u8),
        }
    }
}

impl From<crate::full::FullTrack> for InternalTrack {
    fn from(full_track: crate::full::FullTrack) -> Self {
        let crate::full::FullTrack { track, artist, .. } = full_track;
        InternalTrack {
            title: track.title,
            artists: artist.into_iter().map(|a| a.name).collect(),
            length: Some(track.length),
            disc: None, // TODO: see above
            number: Some(track.number),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cover {
    pub provider: ArtProvider,
    pub url: String,
    pub width: usize,
    pub height: usize,
    pub title: String,
    pub artist: String,
}

// Covers are sorted by picture size
impl Ord for Cover {
    fn cmp(&self, other: &Self) -> Ordering {
        let s1 = self.width * self.height;
        let s2 = other.width * other.height;
        s1.cmp(&s2)
    }
}

impl PartialOrd for Cover {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Cover {
    fn eq(&self, other: &Self) -> bool {
        self.width * self.height == other.width * other.height
    }
}
impl Eq for Cover {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct InternalTracks(pub Vec<InternalTrack>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Artists(pub Vec<super::Artist>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistCredits(pub Vec<super::ArtistCredit>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Releases(pub Vec<super::Release>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Mediums(pub Vec<super::Medium>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Tracks(pub Vec<super::Track>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistTrackRelations(pub Vec<super::ArtistTrackRelation>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistCreditReleases(pub Vec<super::ArtistCreditRelease>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistCreditTracks(pub Vec<super::ArtistCreditTrack>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Covers(pub Vec<Cover>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ReleaseRating(pub i64, pub Vec<usize>);
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ReleaseMatches(pub HashMap<Uuid, ReleaseRating>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct CoverRatings(pub Vec<f32>);

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "import")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub directory: String,
    pub source_release: InternalRelease,
    pub source_tracks: InternalTracks,

    pub artists: Artists,
    pub artist_credits: ArtistCredits,
    pub releases: Releases,
    pub mediums: Mediums,
    pub tracks: Tracks,
    pub artist_track_relations: ArtistTrackRelations,
    pub artist_credit_releases: ArtistCreditReleases,
    pub artist_credit_tracks: ArtistCreditTracks,
    pub covers: Covers,

    pub release_matches: ReleaseMatches,
    pub cover_ratings: CoverRatings,

    pub started_at: time::OffsetDateTime,
    pub ended_at: Option<time::OffsetDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Hash for Column {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.to_string().eq(&other.to_string())
    }
}

impl Eq for Column {}

impl TryFrom<String> for Column {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "id" => Ok(Column::Id),
            "started_at" => Ok(Column::StartedAt),
            "ended_at" => Ok(Column::EndedAt),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}
