use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artist_picture")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub image_id: String,
    #[sea_orm(primary_key)]
    pub artist_id: Uuid,
    #[sea_orm(primary_key)]
    pub r#type: PictureType,
}

#[derive(
    Deserialize,
    Serialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    PartialOrd,
    Ord,
    Hash,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum PictureType {
    #[sea_orm(num_value = 0)]
    #[serde(rename = "picture")]
    Picture,
    #[sea_orm(num_value = 1)]
    #[serde(rename = "cover")]
    Cover,
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
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::Id"
    )]
    Artist,
}

impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
