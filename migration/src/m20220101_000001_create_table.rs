use entity::{ArtistColumn, ArtistEntity};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ArtistEntity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtistColumn::Mbid)
                            .blob(BlobSize::Tiny)
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await
    }
}
