use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "genre_track")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub genre_id: String,
    #[sea_orm(primary_key)]
    pub track_id: Uuid,
    pub cnt: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::genre::Entity",
        from = "Column::GenreId",
        to = "super::genre::Column::Id"
    )]
    Genre,
    #[sea_orm(
        belongs_to = "super::track::Entity",
        from = "Column::TrackId",
        to = "super::track::Column::Id"
    )]
    Track,
}

impl Related<super::genre::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Genre.def()
    }
}

impl Related<super::track::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Track.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.genre_id.eq(&other.genre_id) && self.track_id.eq(&other.track_id)
    }
}
impl Eq for Model {}

impl PartialOrd for Model {
    fn lt(&self, other: &Self) -> bool {
        self.genre_id.lt(&other.genre_id) && self.track_id.lt(&other.track_id)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.genre_id
            .partial_cmp(&other.genre_id)
            .and(self.track_id.partial_cmp(&other.track_id))
    }
}

impl Ord for Model {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.eq(other) {
            std::cmp::Ordering::Equal
        } else {
            self.genre_id.cmp(&other.genre_id)
        }
    }
}
