use std::hash::Hash;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Instruments(Vec<String>);

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "artist")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub sort_name: String,

    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::artist_credit::Entity")]
    ArtistCredit,
    #[sea_orm(has_many = "super::artist_url::Entity")]
    ArtistUrl,
    #[sea_orm(has_many = "super::image_artist::Entity")]
    Image,
    #[sea_orm(has_many = "super::artist_track_relation::Entity")]
    TrackRelation,
    #[sea_orm(has_many = "super::artist_picture::Entity")]
    Picture,
    #[sea_orm(has_many = "super::update_artist::Entity")]
    Update,
}

impl Related<super::artist_credit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistCredit.def()
    }
}

impl Related<super::artist_url::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistUrl.def()
    }
}

impl Related<super::image_artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}

impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        super::image_artist::Relation::Image.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::image_artist::Relation::Artist.def().rev())
    }
}

impl Related<super::artist_track_relation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TrackRelation.def()
    }
}

impl Related<super::update_artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Update.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::update_artist::Relation::Artist.def().rev())
    }
}

impl Related<super::artist_picture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Picture.def()
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
            "name" => Ok(Column::Name),
            "sort_name" => Ok(Column::SortName),
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
