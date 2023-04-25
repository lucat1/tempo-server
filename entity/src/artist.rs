use std::hash::Hash;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Instruments(Vec<String>);

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "artist")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub sort_name: String,
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

impl Related<super::artist_track_relation::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_track_relation::Relation::Track.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_track_relation::Relation::Artist.def().rev())
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
            "name" => Ok(Column::Name),
            "sort_name" => Ok(Column::SortName),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}
