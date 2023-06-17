use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum ConnectionProvider {
    #[sea_orm(num_value = 0)]
    #[serde(rename = "lastfm")]
    LastFM,
}

pub trait Named {
    fn name(&self) -> &'static str;
}

impl Named for ConnectionProvider {
    fn name(&self) -> &'static str {
        match self {
            ConnectionProvider::LastFM => "lastfm",
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct LastFMData {
    pub token: String,
    pub username: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_connection")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub provider: ConnectionProvider,
    pub data: serde_json::Value,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::User",
        to = "super::user::Column::Username"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
