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
pub enum Relation {
    #[sea_orm(has_many = "super::artist_credit::Entity")]
    ArtistCredit,
}

impl Related<super::artist_credit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistCredit.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
