use std::hash::Hash;

use sea_orm::entity::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "scrobble")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub at: TimeDateTimeWithTimeZone,
    pub user: String,
    pub track: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::User",
        to = "super::user::Column::Username"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::track::Entity",
        from = "Column::Track",
        to = "super::track::Column::Id"
    )]
    Track,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Track.def()
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
            "user" => Ok(Column::User),
            "at" => Ok(Column::At),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}
