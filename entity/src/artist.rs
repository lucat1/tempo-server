use sea_orm::entity::prelude::*;
// use serde::{Deserialize, Serialize};
use uuid::Uuid;

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
// pub struct Instruments(Vec<String>);

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "artists")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    // pub instruments: Instruments,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_release::Relation::Release.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_release::Relation::Artist.def().rev())
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_track::Relation::Track.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_track::Relation::Artist.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
