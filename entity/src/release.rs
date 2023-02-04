use chrono::NaiveDate;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "release")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub title: String,
    pub release_group_id: Option<Uuid>,
    pub release_type: Option<String>,
    pub asin: String,
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

impl Related<super::artist_credit::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_credit_release::Relation::ArtistCredit.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::artist_credit_release::Relation::Release.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
