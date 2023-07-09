use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::hash::Hash;

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
            "title" => Ok(Column::Title),
            "description" => Ok(Column::Description),
            "scheduled_at" => Ok(Column::ScheduledAt),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}
