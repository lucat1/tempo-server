use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "image")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    // hash of path
    pub id: String,
    pub format: base::ImageFormat,
    pub width: u32,
    pub height: u32,
    pub size: u32,
    #[sea_orm(primary_key)]
    pub path: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        super::image_release::Relation::Release.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::image_release::Relation::Image.def().rev())
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        super::image_artist::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::image_artist::Relation::Image.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
