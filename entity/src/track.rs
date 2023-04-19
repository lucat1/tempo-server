use super::TrackFormat;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "track")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub medium_id: Uuid,
    pub title: String,
    pub length: u32,
    pub number: u32,
    pub genres: crate::Genres,
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
