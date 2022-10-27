use eyre::{eyre, Report, Result};
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::models::GroupTracks;
use crate::settings::ArtProvider;
use crate::util::maybe_date;
use crate::SETTINGS;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    // pub disambiguation: Option<String>,
    #[serde(rename = "label-info")]
    #[serde(default)]
    pub label_info: Vec<LabelInfo>,
    pub status: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<ReleaseGroup>,
    // #[serde(rename = "status-id")]
    // pub status_id: Option<String>,
    // pub packaging: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    pub asin: Option<String>,
    pub date: Option<String>,
    // #[serde(rename = "release-events")]
    // #[serde(default)]
    // pub release_events: Vec<Event>,
    pub id: String,
    // pub barcode: Option<String>,
    // pub quality: Option<String>,
    pub media: Vec<Medium>,
    pub country: Option<String>,
    // #[serde(rename = "packaging-id")]
    // pub packaging_id: Option<String>,
    #[serde(rename = "text-representation")]
    pub text_representation: Option<TextRepresentation>,
    pub title: String,
    // #[serde(default)]
    // pub tags: Vec<Tag>,
    #[serde(rename = "track-count")]
    pub track_count: Option<usize>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGroup {
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub title: String,
    pub id: String,
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
    pub type_id: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub disambiguation: Option<String>,
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Vec<String>,
    pub id: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Medium {
    pub id: Option<String>,
    pub position: Option<u64>,
    pub track_offset: Option<u64>,
    pub tracks: Option<Vec<Track>>,
    pub format: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub relations: Vec<Relation>,
    pub disambiguation: String,
    pub id: String,
    pub length: Option<u64>,
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
    Engigneer,
    Instrument,
    Performer,
    Mix,
    Producer,
    Vocal,
    Lyricist,
    Writer,
    Composer,

    Performance,
    Other(String),
}

impl From<String> for RelationType {
    fn from(str: String) -> Self {
        match str.as_str() {
            "engigneer" => Self::Engigneer,
            "instrument" => Self::Instrument,
            "performer" => Self::Performer,
            "mix" => Self::Mix,
            "producer" => Self::Producer,
            "vocal" => Self::Vocal,
            "lyricist" => Self::Lyricist,
            "writer" => Self::Writer,
            "composer" => Self::Composer,
            "performance" => Self::Performance,
            _ => Self::Other(str),
        }
    }
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
    pub id: String,
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub count: i64,
    pub name: String,
}

impl From<ArtistCredit> for crate::models::Artist {
    fn from(artist: ArtistCredit) -> Self {
        crate::models::Artist {
            mbid: Some(artist.artist.id),
            join_phrase: artist.joinphrase,
            name: artist.name,
            sort_name: Some(artist.artist.sort_name),
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
            mbid: Some(artist.id),
            join_phrase: None,
            name: artist.name,
            sort_name: Some(artist.sort_name),
            instruments: relation.attributes,
        })
    }
}

fn artists_from_relationships(
    rels: &[Relation],
    ok: Vec<RelationType>,
) -> Vec<crate::models::Artist> {
    rels.iter()
        .filter(|r| ok.contains(&r.type_field.clone().into()))
        .filter_map(|a| a.clone().try_into().ok())
        .collect()
}

impl From<Track> for crate::models::Track {
    fn from(track: Track) -> Self {
        let mut sorted_genres = track.recording.genres.unwrap_or_default();
        sorted_genres.sort_by(|a, b| a.count.partial_cmp(&b.count).unwrap_or(Ordering::Equal));
        let mut other_relations = track
            .recording
            .relations
            .iter()
            .filter_map(|rel| {
                if RelationType::Performance == rel.type_field.clone().into() {
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
                .or(track.recording.length)
                .map(Duration::from_millis),
            disc: track.medium.clone().and_then(|m| m.position),
            disc_mbid: track.medium.and_then(|m| m.id.clone()),
            number: Some(track.position),
            genres: sorted_genres
                .into_iter()
                .map(|g| g.name)
                .collect::<Vec<_>>(),
            release: track.release.map(|r| (*r).clone().into()),

            performers: artists_from_relationships(
                &relations,
                vec![
                    RelationType::Instrument,
                    RelationType::Performer,
                    RelationType::Vocal,
                ],
            ),
            engigneers: artists_from_relationships(&relations, vec![RelationType::Engigneer]),
            mixers: artists_from_relationships(&track.recording.relations, vec![RelationType::Mix]),
            producers: artists_from_relationships(&relations, vec![RelationType::Producer]),
            lyricists: artists_from_relationships(&relations, vec![RelationType::Lyricist]),
            writers: artists_from_relationships(&relations, vec![RelationType::Writer]),
            composers: artists_from_relationships(&relations, vec![RelationType::Composer]),

            format: None,
            path: None,
        }
    }
}

impl From<Release> for crate::models::Release {
    fn from(release: Release) -> Self {
        let original_date = maybe_date(
            release
                .release_group
                .as_ref()
                .and_then(|r| r.first_release_date.clone()),
        );
        crate::models::Release {
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
            media: release.media.first().and_then(|m| m.format.clone()),
            country: release.country,
            label: release
                .label_info
                .first()
                .and_then(|li| li.label.as_ref())
                .map(|l| l.name.clone()),
            catalog_no: release
                .label_info
                .first()
                .and_then(|l| l.catalog_number.clone()),
            status: release.status,
            release_type: release
                .release_group
                .as_ref()
                .and_then(|r| r.primary_type.as_ref().map(|t| t.to_lowercase())),
            date: SETTINGS.get().and_then(|s| {
                if s.tagging.use_original_date {
                    original_date
                } else {
                    maybe_date(release.date)
                }
            }),
            original_date,
            script: release.text_representation.and_then(|t| t.script),
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
            .map(Arc::new)
            .filter_map(|medium| {
                medium.tracks.as_ref().map(|tracks| {
                    tracks
                        .iter()
                        .map(|t| {
                            let mut t_copy = t.clone();
                            t_copy.medium = Some(medium.clone());
                            t_copy.release = Some(self.clone());
                            t_copy
                        })
                        .collect::<Vec<_>>()
                })
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverArtArchive {
    pub images: Vec<Image>,
}

impl CoverArtArchive {
    pub fn into(self, title: String, artist: String) -> Vec<Cover> {
        self.images
            .into_iter()
            .filter_map(|i| {
                if i.front {
                    let sizes: HashMap<usize, String> = i
                        .thumbnails
                        .into_iter()
                        .filter_map(|(k, v)| k.parse::<usize>().ok().map(|d| (d, v)))
                        .collect();
                    sizes.keys().max().and_then(|size| {
                        sizes.get(size).map(|url| Cover {
                            provider: ArtProvider::CoverArtArchive,
                            url: url.to_string(),
                            width: *size,
                            height: *size,
                            title: title.clone(),
                            artist: artist.clone(),
                        })
                    })
                } else {
                    None
                }
            })
            .collect()
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Itunes {
    pub results: Vec<ItunesResult>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItunesResult {
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "artworkUrl100")]
    pub artwork_url_100: String,
    pub max_size: Option<usize>,
}

impl From<Itunes> for Vec<Cover> {
    fn from(caa: Itunes) -> Self {
        caa.results
            .into_iter()
            .filter_map(|i| {
                i.max_size.map(|s| Cover {
                    provider: ArtProvider::Itunes,
                    url: i
                        .artwork_url_100
                        .replace("100x100", format!("{}x{}", s, s).as_str()),
                    width: s,
                    height: s,
                    title: i.collection_name,
                    artist: i.artist_name,
                })
            })
            .collect()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    approved: bool,
    front: bool,
    thumbnails: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
