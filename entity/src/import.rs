use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Artists(pub Vec<super::Artist>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistCredits(pub Vec<super::ArtistCredit>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Releases(pub Vec<super::Release>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Mediums(pub Vec<super::Medium>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Tracks(pub Vec<super::Track>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistCreditsReleases(pub Vec<super::ArtistCreditRelease>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ArtistCreditsTracks(pub Vec<super::ArtistCreditTrack>);

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "import")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    pub artsits: Artists,
    pub artist_credits: ArtistCredits,
    pub releases: Releases,
    pub mediums: Mediums,
    pub tracks: Tracks,
    pub artist_credits_releases: ArtistCreditsReleases,
    pub artist_credits_tracks: ArtistCreditsTracks,

    pub job: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::job::Entity",
        from = "Column::Job",
        to = "super::job::Column::Id"
    )]
    Job,
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
