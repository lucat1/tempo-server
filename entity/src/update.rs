use sea_orm::entity::prelude::*;
use sea_query::SimpleExpr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "update")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub r#type: UpdateType,
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid_id: uuid::Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub string_id: String,

    pub time: time::OffsetDateTime,
}

#[derive(
    Serialize,
    Deserialize,
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
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum UpdateType {
    #[sea_orm(string_value = "artist_description")]
    #[serde(rename = "artist_description")]
    ArtistDescription,

    #[sea_orm(string_value = "urls")]
    #[serde(rename = "urls")]
    URLs,

    #[sea_orm(string_value = "lastfm_artist_image")]
    #[serde(rename = "lastfm_artist_image")]
    LastFMArtistImage,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::UuidId",
        to = "super::artist::Column::Id"
    )]
    Artist,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub fn filter_condition(before: time::OffsetDateTime, r#type: UpdateType) -> SimpleExpr {
    Column::Time
        .lte(before)
        .and(Column::Type.eq(r#type))
        .or(Column::Time.is_null().and(Column::Type.is_null()))
}
