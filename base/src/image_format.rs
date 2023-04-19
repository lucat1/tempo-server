use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, EnumIter, DeriveActiveEnum, Eq,
)]
#[sea_orm(rs_type = "i8", db_type = "Integer")]
pub enum ImageFormat {
    #[default]
    #[sea_orm(num_value = 0)]
    Jpeg,
    #[sea_orm(num_value = 1)]
    Png,
}
