use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Artist {
    Table,
    #[iden = "mbid"]
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Artist::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Artist::Id)
                            .blob(BlobSize::Tiny)
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await
    }
}
