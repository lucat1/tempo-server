use entity::{ReleaseColumn, ReleaseEntity};
use sea_orm::ColumnTrait;
use sea_orm_migration::prelude::*;
use sea_query::Table;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut binding = Table::alter();
        let table =
            binding
                .table(ReleaseEntity)
                .add_column_if_not_exists(&mut ColumnDef::new_with_type(
                    ReleaseColumn::Disambiguation,
                    ReleaseColumn::Disambiguation
                        .def()
                        .get_column_type()
                        .clone(),
                ));
        manager.alter_table(table.to_owned()).await?;
        Ok(())
    }
}
