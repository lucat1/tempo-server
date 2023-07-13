use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "artists_credit_release")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub artist_credit_id: String,
    #[sea_orm(primary_key)]
    pub release_id: Uuid,
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
        belongs_to = "super::release::Entity",
        from = "Column::ReleaseId",
        to = "super::release::Column::Id"
    )]
    Release,
}

impl Related<super::artist_credit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArtistCredit.def()
    }
}

impl Related<super::release::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Release.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.artist_credit_id.eq(&other.artist_credit_id) && self.release_id.eq(&other.release_id)
    }
}
impl Eq for Model {}

impl PartialOrd for Model {
    fn lt(&self, other: &Self) -> bool {
        self.artist_credit_id.lt(&other.artist_credit_id) && self.release_id.lt(&other.release_id)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.artist_credit_id
            .partial_cmp(&other.artist_credit_id)
            .and(self.release_id.partial_cmp(&other.release_id))
    }
}

impl Ord for Model {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.eq(&other) {
            std::cmp::Ordering::Equal
        } else {
            self.artist_credit_id.cmp(&other.artist_credit_id)
        }
    }
}
