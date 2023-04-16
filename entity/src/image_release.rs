use sea_orm::entity::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "image_release")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub image_id: String,
    #[sea_orm(primary_key)]
    pub release_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::image::Entity",
        from = "Column::ImageId",
        to = "super::image::Column::Id"
    )]
    Image,
    #[sea_orm(
        belongs_to = "super::release::Entity",
        from = "Column::ReleaseId",
        to = "super::release::Column::Id"
    )]
    Release,
}

impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Release.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
