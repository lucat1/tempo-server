use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artists_credit")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub join_phrase: Option<String>,
    pub artist_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::Id"
    )]
    Artist,

    #[sea_orm(has_many = "super::artist_credit_release::Entity")]
    ArtistCreditRelease,
    #[sea_orm(has_many = "super::artist_credit_track::Entity")]
    ArtistCreditTrack,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl Related<super::artist_credit_release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistCreditRelease.def()
    }
}

impl Related<super::artist_credit_track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistCreditTrack.def()
    }
}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_credit_release::Relation::Release.def()
    }

    fn via() -> Option<RelationDef> {
        Some(
            super::artist_credit_release::Relation::ArtistCredit
                .def()
                .rev(),
        )
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        super::artist_credit_track::Relation::Track.def()
    }

    fn via() -> Option<RelationDef> {
        Some(
            super::artist_credit_track::Relation::ArtistCredit
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
