use std::hash::Hash;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "release")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub title: String,
    pub disambiguation: Option<String>,
    pub release_group_id: Option<Uuid>,
    pub release_type: Option<String>,
    pub asin: Option<String>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub catalog_no: Option<String>,
    pub status: Option<String>,
    pub year: Option<i32>,
    pub month: Option<i16>,
    pub day: Option<i16>,
    pub original_year: Option<i32>,
    pub original_month: Option<i16>,
    pub original_day: Option<i16>,
    pub script: Option<String>,

    pub path: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::medium::Entity")]
    Medium,
    #[sea_orm(has_one = "super::image_release::Entity")]
    Image,
    #[sea_orm(has_many = "super::genre::Entity")]
    Genre,
    #[sea_orm(has_many = "super::genre_release::Entity")]
    GenreRelease,
}

impl Related<super::medium::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Medium.def()
    }
}

impl Related<super::image_release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
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

impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        super::image_release::Relation::Image.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::image_release::Relation::Release.def().rev())
    }
}

impl Related<super::genre::Entity> for Entity {
    fn to() -> RelationDef {
        super::genre_release::Relation::Genre.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::genre_release::Relation::Release.def().rev())
    }
}

impl Related<super::genre_release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GenreRelease.def()
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
            "release-group-id" => Ok(Column::ReleaseGroupId),
            "release-type" => Ok(Column::ReleaseType),
            "asin" => Ok(Column::Asin),
            "country" => Ok(Column::Country),
            "label" => Ok(Column::Label),
            "catalog-no" => Ok(Column::CatalogNo),
            "status" => Ok(Column::Status),
            "year" => Ok(Column::Year),
            "month" => Ok(Column::Month),
            "day" => Ok(Column::Day),
            "original_year" => Ok(Column::OriginalYear),
            "original_month" => Ok(Column::OriginalMonth),
            "original_day" => Ok(Column::OriginalDay),
            "script" => Ok(Column::Script),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl Eq for Model {}

impl Ord for Model {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Model {
    fn lt(&self, other: &Self) -> bool {
        self.id.lt(&other.id)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
