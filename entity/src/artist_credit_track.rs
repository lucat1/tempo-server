use sea_orm::entity::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artists_credit_track")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub artist_credit_id: String,
    #[sea_orm(primary_key)]
    pub track_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist_credit::Entity",
        from = "Column::ArtistCreditId",
        to = "super::artist_credit::Column::Id"
    )]
    ArtistCredit,
    #[sea_orm(
        belongs_to = "super::track::Entity",
        from = "Column::TrackId",
        to = "super::track::Column::Id"
    )]
    Track,
}

impl ActiveModelBehavior for ActiveModel {}
