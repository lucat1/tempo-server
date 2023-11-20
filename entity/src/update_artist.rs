use sea_orm::entity::prelude::*;
use sea_query::SimpleExpr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "update_artist")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub r#type: UpdateType,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: uuid::Uuid,

    pub time: time::OffsetDateTime,
}

#[derive(
    Deserialize,
    Serialize,
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
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum UpdateType {
    #[sea_orm(num_value = 0)]
    #[serde(rename = "artist_description")]
    ArtistDescription,

    #[sea_orm(num_value = 1)]
    #[serde(rename = "artist_url")]
    ArtistUrl,

    #[sea_orm(num_value = 2)]
    #[serde(rename = "lastfm_artist_image")]
    LastFMArtistImage,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::Id",
        to = "super::artist::Column::Id"
    )]
    Artist,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

pub fn filter(r#type: UpdateType, before: time::OffsetDateTime) -> SimpleExpr {
    Column::Time
        .lte(before)
        .and(Column::Type.eq(r#type))
        .or(Column::Time.is_null().and(Column::Type.is_null()))
}

impl ActiveModelBehavior for ActiveModel {}
