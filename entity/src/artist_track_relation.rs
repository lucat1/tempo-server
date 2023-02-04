use sea_orm::entity::prelude::*;
use sea_orm::TryFromU64;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i8", db_type = "Integer")]
pub enum RelationType {
    #[sea_orm(num_value = 0)]
    Engigneer,
    #[sea_orm(num_value = 1)]
    Instrument,
    #[sea_orm(num_value = 2)]
    Performer,
    #[sea_orm(num_value = 3)]
    Mix,
    #[sea_orm(num_value = 4)]
    Producer,
    #[sea_orm(num_value = 5)]
    Vocal,
    #[sea_orm(num_value = 6)]
    Lyricist,
    #[sea_orm(num_value = 7)]
    Writer,
    #[sea_orm(num_value = 8)]
    Composer,
    #[sea_orm(num_value = 9)]
    Performance,
    #[sea_orm(num_value = 10)]
    Other,
}

impl From<String> for RelationType {
    fn from(str: String) -> Self {
        match str.as_str() {
            "engigneer" => Self::Engigneer,
            "instrument" => Self::Instrument,
            "performer" => Self::Performer,
            "mix" => Self::Mix,
            "producer" => Self::Producer,
            "vocal" => Self::Vocal,
            "lyricist" => Self::Lyricist,
            "writer" => Self::Writer,
            "composer" => Self::Composer,
            "performance" => Self::Performance,
            _ => Self::Other,
        }
    }
}

// TODO: remove once:
// - https://github.com/SeaQL/sea-orm/issues/1364 is closed
// - https://github.com/SeaQL/sea-orm/pull/1414 is merged
impl TryFromU64 for RelationType {
    fn try_from_u64(_: u64) -> Result<Self, DbErr> {
        Err(DbErr::ConvertFromU64(
            "Fail to construct ActiveEnum from a u64, if your primary key consist of a ActiveEnum field, its auto increment should be set to false."
        ))
    }
}

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "arist_track_relation"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveModel, DeriveActiveModel)]
pub struct Model {
    pub artist_id: Uuid,
    pub track_id: Uuid,
    pub relation_type: RelationType,
    pub relation_value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    ArtistId,
    TrackId,
    RelationType,
    RelationValue,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    ArtistId,
    TrackId,
    RelationType,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = (Uuid, Uuid, RelationType);

    fn auto_increment() -> bool {
        false
    }
}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            Self::ArtistId => ColumnType::Uuid.def(),
            Self::TrackId => ColumnType::Uuid.def(),
            Self::RelationType => ColumnType::Json.def(),
            Self::RelationValue => ColumnType::String(None).def(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::Id"
    )]
    Artist,
    #[sea_orm(
        belongs_to = "super::track::Entity",
        from = "Column::TrackId",
        to = "super::track::Column::Id"
    )]
    Track,
}

impl ActiveModelBehavior for ActiveModel {}