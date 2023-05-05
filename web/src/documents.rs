use serde::Serialize;

#[derive(Serialize)]
pub struct ArtistAttributes {
    pub name: String,
    pub sort_name: String,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtistRelation {
    #[serde(rename = "images")]
    Image,
    #[serde(rename = "artist_credits")]
    ArtistCredit,
}

#[derive(Serialize)]
pub struct ArtistCreditAttributes {
    pub join_phrase: Option<String>,
}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtistCreditRelation {
    Artist,
}

#[derive(Serialize)]
pub struct MediumAttributes {}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediumRelation {
    Release,
    Tracks,
}

#[derive(Serialize)]
pub struct ReleaseAttributes {}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseRelation {
    Mediums,
    Artists,
}

#[derive(Serialize)]
pub struct TrackAttributes {}

#[derive(Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackRelation {
    Artists,
    Medium,
}
