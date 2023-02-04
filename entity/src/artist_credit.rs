use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artists_credit")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: u64,
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
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
