use chrono::NaiveDate;
use eyre::{eyre, Report, Result};
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::models::GroupTracks;
use crate::SETTINGS;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub disambiguation: Option<String>,
    #[serde(rename = "label-info")]
    #[serde(default)]
    pub label_info: Vec<LabelInfo>,
    pub status: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<ReleaseGroup>,
    #[serde(rename = "status-id")]
    pub status_id: Option<String>,
    pub packaging: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    pub asin: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "release-events")]
    #[serde(default)]
    pub release_events: Vec<Event>,
    pub id: String,
    pub barcode: Option<String>,
    pub quality: Option<String>,
    pub media: Vec<Medium>,
    pub country: Option<String>,
    #[serde(rename = "packaging-id")]
    pub packaging_id: Option<String>,
    #[serde(rename = "text-representation")]
    pub text_representation: Option<TextRepresentation>,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(rename = "track-count")]
    pub track_count: Option<usize>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
    pub name: String,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGroup {
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: String,
    #[serde(rename = "primary-type-id")]
    pub primary_type_id: String,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "primary-type")]
    pub primary_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub joinphrase: Option<String>,
    pub artist: Artist,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub disambiguation: Option<String>,
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub area: Area,
    pub date: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Area {
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Vec<String>,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Medium {
    pub id: Option<String>,
    pub position: Option<u64>,
    pub track_offset: Option<u64>,
    pub tracks: Option<Vec<Track>>,
    pub format: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub recording: Recording,
    pub number: String,
    pub position: u64,
    pub length: Option<u64>,
    pub title: String,

    pub medium: Option<Arc<Medium>>,
    pub release: Option<Arc<Release>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recording {
    pub relations: Vec<Relation>,
    pub disambiguation: String,
    pub id: String,
    pub length: u64,
    pub video: bool,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: Option<String>,
    pub genres: Option<Vec<Genre>>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationType {
    ENGIGNEER,
    INSTRUMENT,
    PERFORMER,
    MIX,
    PRODUCER,
    VOCAL,
    LYRICIST,
    WRITER,
    COMPOSER,

    PERFORMANCE,
    OTHER(String),
}

impl From<String> for RelationType {
    fn from(str: String) -> Self {
        match str.as_str() {
            "engigneer" => Self::ENGIGNEER,
            "instrument" => Self::INSTRUMENT,
            "performer" => Self::PERFORMER,
            "mix" => Self::MIX,
            "producer" => Self::PRODUCER,
            "vocal" => Self::VOCAL,
            "lyricist" => Self::LYRICIST,
            "writer" => Self::WRITER,
            "composer" => Self::COMPOSER,
            "performance" => Self::PERFORMANCE,
            _ => Self::OTHER(str),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "type")]
    pub type_field: String,
    pub artist: Option<Artist>,
    pub work: Option<Work>,
    pub attributes: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Work {
    pub relations: Option<Vec<Relation>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Genre {
    pub id: String,
    pub count: u64,
    pub disambiguation: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseSearch {
    pub created: String,
    pub count: i64,
    pub offset: i64,
    pub releases: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRepresentation {
    pub language: Option<String>,
    pub script: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
    pub label: Option<Label>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub count: i64,
    pub name: String,
}

impl From<ArtistCredit> for crate::models::Artist {
    fn from(artist: ArtistCredit) -> Self {
        crate::models::Artist {
            mbid: Some(artist.artist.id.clone()),
            join_phrase: artist.joinphrase.clone(),
            name: artist.name.clone(),
            sort_name: Some(artist.artist.sort_name.clone()),
            instruments: vec![],
        }
    }
}

impl TryFrom<Relation> for crate::models::Artist {
    type Error = Report;
    fn try_from(relation: Relation) -> Result<Self> {
        let artist = relation
            .artist
            .ok_or(eyre!("Relation doesn't contain an artist"))?;
        Ok(crate::models::Artist {
            mbid: Some(artist.id.clone()),
            join_phrase: None,
            name: artist.name.clone(),
            sort_name: Some(artist.sort_name.clone()),
            instruments: relation.attributes,
        })
    }
}

fn artists_from_relationships(
    rels: &Vec<Relation>,
    ok: Vec<RelationType>,
) -> Vec<crate::models::Artist> {
    rels.iter()
        .filter(|r| ok.contains(&r.type_field.clone().into()))
        .filter_map(|a| a.clone().try_into().ok())
        .collect()
}

impl From<Track> for crate::models::Track {
    fn from(track: Track) -> Self {
        let mut sorted_genres = track.recording.genres.unwrap_or(vec![]);
        sorted_genres.sort_by(|a, b| a.count.partial_cmp(&b.count).unwrap_or(Ordering::Equal));
        let mut other_relations = track
            .recording
            .relations
            .iter()
            .filter_map(|rel| {
                if RelationType::PERFORMANCE == rel.type_field.clone().into() {
                    rel.work.clone()
                } else {
                    None
                }
            })
            .filter_map(|work| work.relations)
            .flatten()
            .collect::<Vec<_>>();
        let mut relations = track.recording.relations.clone();
        relations.append(&mut other_relations);

        crate::models::Track {
            mbid: Some(track.id),
            title: track.title,
            artists: track.recording.artist_credit.map_or(vec![], |artists| {
                artists.into_iter().map(|a| a.into()).collect()
            }),
            length: track
                .length
                .or(Some(track.recording.length))
                .map(|d| Duration::from_millis(d)),
            disc: track.medium.clone().map_or(None, |m| m.position),
            disc_mbid: track.medium.map_or(None, |m| m.id.clone()),
            number: Some(track.position),
            genres: sorted_genres
                .into_iter()
                .map(|g| g.name)
                .collect::<Vec<_>>(),
            release: track.release.map(|r| Arc::new((*r).clone().into())),

            performers: artists_from_relationships(
                &relations,
                vec![
                    RelationType::INSTRUMENT,
                    RelationType::PERFORMER,
                    RelationType::VOCAL,
                ],
            ),
            engigneers: artists_from_relationships(&relations, vec![RelationType::ENGIGNEER]),
            mixers: artists_from_relationships(&track.recording.relations, vec![RelationType::MIX]),
            producers: artists_from_relationships(&relations, vec![RelationType::PRODUCER]),
            lyricists: artists_from_relationships(&relations, vec![RelationType::LYRICIST]),
            writers: artists_from_relationships(&relations, vec![RelationType::WRITER]),
            composers: artists_from_relationships(&relations, vec![RelationType::COMPOSER]),
        }
    }
}

fn maybe_date(d: Option<String>) -> Option<NaiveDate> {
    d.map_or(None, |s| {
        NaiveDate::parse_from_str(s.as_str(), "%Y-%m-%d")
            .ok()
            .or(NaiveDate::parse_from_str(s.as_str(), "%Y").ok())
    })
}

impl From<Release> for crate::models::Release {
    fn from(release: Release) -> Self {
        let original_date = maybe_date(
            release
                .release_group
                .as_ref()
                .map_or(None, |r| r.first_release_date.clone()),
        );
        crate::models::Release {
            // TODO: no good
            mbid: Some(release.id),
            release_group_mbid: release.release_group.as_ref().map(|r| r.id.clone()),
            asin: release.asin,
            title: release.title,
            tracks: Some(
                release
                    .media
                    .iter()
                    .map(|m| m.tracks.as_ref().map_or(0, |tracks| tracks.len() as u64))
                    .sum(),
            ),
            discs: Some(release.media.len() as u64),
            media: release.media.first().map_or(None, |m| m.format.clone()),
            country: release.country,
            label: release
                .label_info
                .first()
                .map_or(None, |li| li.label.as_ref())
                .map(|l| l.name.clone()),
            catalog_no: release
                .label_info
                .first()
                .map_or(None, |l| l.catalog_number.clone()),
            status: release.status,
            release_type: release
                .release_group
                .as_ref()
                .map(|r| r.primary_type.clone()),
            date: SETTINGS.get().map_or(None, |s| {
                if s.tagging.use_original_date {
                    original_date
                } else {
                    maybe_date(release.date)
                }
            }),
            original_date,
            script: release.text_representation.map_or(None, |t| t.script),
            artists: release
                .artist_credit
                .into_iter()
                .map(|a| a.into())
                .collect::<Vec<_>>(),
        }
    }
}

impl GroupTracks for Arc<Release> {
    fn group_tracks(self) -> Result<(crate::models::Release, Vec<crate::models::Track>)> {
        let tracks = self
            .media
            .clone()
            .into_iter()
            .map(|m| Arc::new(m))
            .filter_map(|medium| match medium.tracks {
                Some(ref tracks) => Some(
                    tracks
                        .into_iter()
                        .map(|t| {
                            let mut t_copy = t.clone();
                            t_copy.medium = Some(medium.clone());
                            t_copy.release = Some(self.clone());
                            t_copy
                        })
                        .collect::<Vec<_>>(),
                ),
                None => None,
            })
            .flatten()
            .map(|t| t.into())
            .collect::<Vec<_>>();
        Ok((
            Arc::try_unwrap(self)
                .map_err(|_| eyre!("Could not take ownership of Arc<Release>"))?
                .try_into()?,
            tracks,
        ))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverArtArchive {
    pub images: Vec<Image>,
}

impl TryFrom<CoverArtArchive> for String {
    type Error = Report;
    fn try_from(caa: CoverArtArchive) -> Result<Self, Self::Error> {
        caa.images
            .into_iter()
            .filter(|i| i.front)
            .next()
            .ok_or(eyre!("No front covers found"))
            .map(|i| i.image)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Itunes {
    pub results: Vec<ItunesResult>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItunesResult {
    // pub artistName: String,
    // pub collectionName: String,
    #[serde(rename = "artworkUrl100")]
    pub artwork_url_100: String,
}

impl TryFrom<Itunes> for String {
    type Error = Report;
    fn try_from(caa: Itunes) -> Result<Self, Self::Error> {
        caa.results
            .first()
            .ok_or(eyre!("No front covers found"))
            .map(|i| i.artwork_url_100.replace("100x100", "1200x1200"))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    approved: bool,
    back: bool,
    comment: String,
    edit: i64,
    front: bool,
    id: i64,
    image: String,
    thumbnails: Thumbnails,
    types: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thumbnails {
    #[serde(rename = "250")]
    the_250: String,
    #[serde(rename = "500")]
    the_500: String,
    #[serde(rename = "1200")]
    the_1200: String,
    large: String,
    small: String,
}