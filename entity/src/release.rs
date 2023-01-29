use chrono::NaiveDate;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "releases")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub release_group_id: Option<String>,
    pub release_type: Option<String>,
    pub asin: Option<String>,
    pub title: String,
    pub discs: Option<u64>,
    pub media: Option<String>,
    pub tracks: Option<u64>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub catalog_no: Option<String>,
    pub status: Option<String>,
    pub date: Option<NaiveDate>,
    pub original_date: Option<NaiveDate>,
    pub script: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::medium::Entity")]
    Medium,
}

impl Related<super::medium::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Medium.def()
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_release::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_release::Relation::Release.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
