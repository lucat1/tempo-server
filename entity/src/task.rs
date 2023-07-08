use sea_orm::entity::prelude::*;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "task")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    pub data: Value,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// impl Related<super::release::Entity> for Entity {
//     fn to() -> RelationDef {
//         super::image_release::Relation::Release.def()
//     }
//
//     fn via() -> Option<RelationDef> {
//         Some(super::image_release::Relation::Image.def().rev())
//     }
// }

impl ActiveModelBehavior for ActiveModel {}
