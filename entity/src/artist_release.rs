use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artists_releases")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub artist_mbid: Uuid,
    #[sea_orm(primary_key)]
    pub release_mbid: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistMbid",
        to = "super::artist::Column::Mbid"
    )]
    Artist,
    #[sea_orm(
        belongs_to = "super::release::Entity",
        from = "Column::ReleaseMbid",
        to = "super::release::Column::Mbid"
    )]
    Release,
}

impl ActiveModelBehavior for ActiveModel {}
