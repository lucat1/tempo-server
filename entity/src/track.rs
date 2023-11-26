use std::hash::Hash;

use super::TrackFormat;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "track")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub medium_id: Uuid,
    pub title: String,
    pub length: i32,
    pub number: i32,
    pub recording_id: Uuid,

    pub format: Option<TrackFormat>,
    pub path: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::medium::Entity",
        from = "Column::MediumId",
        to = "super::medium::Column::Id"
    )]
    Medium,
    #[sea_orm(has_many = "super::artist_track_relation::Entity")]
    ArtistRelation,
    #[sea_orm(has_many = "super::genre::Entity")]
    Genre,
}

impl Related<super::medium::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Medium.def()
    }
}

impl Related<super::artist_credit::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_credit_track::Relation::ArtistCredit.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_credit_track::Relation::Track.def().rev())
    }
}

impl Related<super::artist_track_relation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistRelation.def()
    }
}

impl Related<super::genre::Entity> for Entity {
    fn to() -> RelationDef {
        super::genre_track::Relation::Genre.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::genre_track::Relation::Track.def().rev())
    }
}

#[derive(Debug)]
pub struct TrackToRelease;

impl Linked for TrackToRelease {
    type FromEntity = super::track::Entity;

    type ToEntity = super::release::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::track::Relation::Medium.def(),
            super::medium::Relation::Release.def(),
        ]
    }
}

#[derive(Debug)]
pub struct TrackToArtist;

impl Linked for TrackToArtist {
    type FromEntity = super::track::Entity;

    type ToEntity = super::artist::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::artist_credit_track::Relation::Track.def(),
            super::artist_credit_track::Relation::ArtistCredit.def(),
            super::artist_credit::Relation::Artist.def(),
        ]
    }
}

#[derive(Debug)]
pub struct TrackToPerformer;

impl Linked for TrackToPerformer {
    type FromEntity = super::track::Entity;

    type ToEntity = super::artist::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::artist_track_relation::Relation::Track.def(),
            super::artist_track_relation::Relation::Artist.def(),
        ]
    }
}

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
            "title" => Ok(Column::Title),
            "duration" => Ok(Column::Length),
            "number" => Ok(Column::Number),
            "recording_mbid" => Ok(Column::RecordingId),
            "mimetype" => Ok(Column::Format),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl Eq for Model {}

impl PartialOrd for Model {
    fn lt(&self, other: &Self) -> bool {
        self.id.lt(&other.id)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for Model {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
