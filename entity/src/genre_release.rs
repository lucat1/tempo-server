use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "genre_release")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub genre_id: String,
    #[sea_orm(primary_key)]
    pub release_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::genre::Entity",
        from = "Column::GenreId",
        to = "super::genre::Column::Id"
    )]
    Genre,
    #[sea_orm(
        belongs_to = "super::release::Entity",
        from = "Column::ReleaseId",
        to = "super::release::Column::Id"
    )]
    Release,
}

impl Related<super::genre::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Genre.def()
    }
}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Release.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
