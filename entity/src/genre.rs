use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "genre")]
pub struct Model {
    // hash of the disambiguation
    #[sea_orm(primary_key, auto_increment = false, column_type = "String(Some(64))")]
    pub id: String,
    pub name: String,
    pub disambiguation: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::genre_release::Entity")]
    GenreRelease,
    #[sea_orm(has_many = "super::genre_track::Entity")]
    GenreTrack,
}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        super::genre_release::Relation::Release.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::genre_release::Relation::Genre.def().rev())
    }
}

impl Related<super::genre_release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GenreRelease.def()
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

impl Related<super::genre_track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GenreTrack.def()
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
            "disambiguation" => Ok(Column::Disambiguation),
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

impl Ord for Model {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Model {
    fn lt(&self, other: &Self) -> bool {
        self.id.lt(&other.id)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
