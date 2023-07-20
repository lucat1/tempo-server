use sea_orm::entity::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::hash::Hash;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "task")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub data: Value,
    pub description: Option<String>,

    pub scheduled_at: time::OffsetDateTime,
    pub started_at: Option<time::OffsetDateTime>,
    pub ended_at: Option<time::OffsetDateTime>,

    pub job: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::job::Entity",
        from = "Column::Job",
        to = "super::job::Column::Id"
    )]
    Job,
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl Related<Entity> for Entity {
    fn to() -> RelationDef {
        super::task_dep_task::Relation::ChildTask.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::task_dep_task::Relation::ChildTask.def().rev())
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
            "description" => Ok(Column::Description),
            "scheduled_at" => Ok(Column::ScheduledAt),
            "started_at" => Ok(Column::StartedAt),
            "ended_at" => Ok(Column::EndedAt),
            "job" => Ok(Column::Job),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}
