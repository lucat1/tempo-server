use super::TrackFormat;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Genres(Vec<String>);

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tracks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub title: String,
    pub length: u64,
    pub number: u64,
    pub genres: Genres,

    pub format: Option<TrackFormat>,
    pub path: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::medium::Entity",
        from = "Column::Id",
        to = "super::medium::Column::Id"
    )]
    Medium,
}

impl Related<super::medium::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Medium.def()
    }
}

impl Related<super::artist_credit::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_credit_track::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_credit_track::Relation::Track.def().rev())
    }
}

impl Related<super::artist_track_relation::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_track_relation::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_track_relation::Relation::Track.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
