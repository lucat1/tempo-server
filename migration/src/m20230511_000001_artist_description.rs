use entity::{ArtistColumn, ArtistEntity};
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
                .table(ArtistEntity)
                .add_column_if_not_exists(&mut ColumnDef::new_with_type(
                    ArtistColumn::Description,
                    ArtistColumn::Description.def().get_column_type().clone(),
                ));
        manager.alter_table(table.to_owned()).await?;
        Ok(())
    }
}
