use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i8", db_type = "Integer")]
pub enum TrackFormat {
    #[sea_orm(num_value = 0)]
    Flac,
    #[sea_orm(num_value = 1)]
    Mp4,
    #[sea_orm(num_value = 2)]
    Id3,
    #[sea_orm(num_value = 3)]
    Ape,
}
