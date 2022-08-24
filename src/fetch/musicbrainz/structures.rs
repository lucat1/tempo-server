use crate::album::ReleaseLike;
use eyre::Result;
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseSearch {
    pub created: String,
    pub count: i64,
    pub offset: i64,
    pub releases: Vec<Release>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub id: String,
    pub score: i64,
    #[serde(rename = "status-id")]
    pub status_id: String,
    pub count: i64,
    pub title: String,
    pub status: String,
    pub disambiguation: Option<String>,
    #[serde(rename = "text-representation")]
    pub text_representation: TextRepresentation,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<ArtistCredit>,
    #[serde(rename = "release-group")]
    pub release_group: ReleaseGroup,
    pub barcode: Option<String>,
    #[serde(rename = "track-count")]
    pub track_count: usize,
    pub media: Vec<Medium>,
    pub date: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "release-events")]
    #[serde(default)]
    pub release_events: Vec<Event>,
    #[serde(rename = "label-info")]
    #[serde(default)]
    pub label_info: Vec<LabelInfo>,
    #[serde(rename = "packaging-id")]
    pub packaging_id: Option<String>,
    pub packaging: Option<String>,
    pub asin: Option<String>,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRepresentation {
    pub language: String,
    pub script: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub artist: Artist,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseGroup {
    pub id: String,
    #[serde(rename = "type-id")]
    pub type_id: String,
    #[serde(rename = "primary-type-id")]
    pub primary_type_id: String,
    pub title: String,
    #[serde(rename = "primary-type")]
    pub primary_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Medium {
    pub format: String,
    #[serde(rename = "disc-count")]
    pub disc_count: i64,
    #[serde(rename = "track-count")]
    pub track_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub date: String,
    pub area: Area,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
    pub label: Option<Label>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub count: i64,
    pub name: String,
}

impl ReleaseLike for Release {
    fn artist(&self) -> Result<Vec<String>> {
        Ok(self
            .artist_credit
            .iter()
            .map(|artist| artist.name.clone())
            .collect::<Vec<_>>())
    }

    fn title(&self) -> Result<Vec<String>> {
        Ok(vec![self.title.clone()])
    }
}
