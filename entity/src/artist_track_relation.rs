use sea_orm::entity::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum RelationType {
    #[sea_orm(string_value = "a")]
    #[serde(rename = "engineer")]
    Engineer,
    #[sea_orm(string_value = "b")]
    #[serde(rename = "instrument")]
    Instrument,
    #[sea_orm(string_value = "c")]
    #[serde(rename = "performer")]
    Performer,
    #[sea_orm(string_value = "d")]
    #[serde(rename = "mix")]
    Mix,
    #[sea_orm(string_value = "e")]
    #[serde(rename = "producer")]
    Producer,
    #[sea_orm(string_value = "f")]
    #[serde(rename = "vocal")]
    Vocal,
    #[sea_orm(string_value = "g")]
    #[serde(rename = "lyricist")]
    Lyricist,
    #[sea_orm(string_value = "h")]
    #[serde(rename = "writer")]
    Writer,
    #[sea_orm(string_value = "i")]
    #[serde(rename = "composer")]
    Composer,
    #[sea_orm(string_value = "j")]
    #[serde(rename = "performance")]
    Performance,
    #[sea_orm(string_value = "k")]
    #[serde(rename = "other")]
    Other,
}

impl From<String> for RelationType {
    fn from(str: String) -> Self {
        match str.as_str() {
            "engineer" => Self::Engineer,
            "instrument" => Self::Instrument,
            "performer" => Self::Performer,
            "mix" => Self::Mix,
            "producer" => Self::Producer,
            "vocal" => Self::Vocal,
            "lyricist" => Self::Lyricist,
            "writer" => Self::Writer,
            "composer" => Self::Composer,
            "performance" => Self::Performance,
            _ => Self::Other,
        }
    }
}

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "artists_track_relation"
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub artist_id: Uuid,
    #[sea_orm(primary_key)]
    pub track_id: Uuid,
    #[sea_orm(primary_key)]
    pub relation_type: RelationType,
    #[sea_orm(primary_key)]
    pub relation_value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::Id"
    )]
    Artist,
    #[sea_orm(
        belongs_to = "super::track::Entity",
        from = "Column::TrackId",
        to = "super::track::Column::Id"
    )]
    Track,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Track.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
