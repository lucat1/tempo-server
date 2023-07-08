use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "job")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_at: time::OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::task::Entity")]
    Task,
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
