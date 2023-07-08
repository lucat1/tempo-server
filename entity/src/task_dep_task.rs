use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "task_dep_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub parent_task: i64,
    #[sea_orm(primary_key)]
    pub child_task: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::task::Entity",
        from = "Column::ParentTask",
        to = "super::task::Column::Id"
    )]
    ParentTask,
    #[sea_orm(
        belongs_to = "super::task::Entity",
        from = "Column::ChildTask",
        to = "super::task::Column::Id"
    )]
    ChildTask,
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ParentTask.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
