use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "genre")]
pub struct Model {
    // hash of the disambiguation
    #[sea_orm(primary_key, auto_increment = false, column_type = "String(Some(64))")]
    pub id: String,
    pub name: String,
    pub disambiguation: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        super::genre_release::Relation::Release.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::genre_release::Relation::Genre.def().rev())
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        super::genre_track::Relation::Track.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::genre_track::Relation::Genre.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
