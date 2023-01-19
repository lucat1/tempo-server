use entity::{ArtistEntity, ArtistReleaseEntity, ReleaseEntity};
use sea_orm::Schema;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let builder = manager.get_database_backend();
        let schema = Schema::new(builder);
        manager
            .exec_stmt(schema.create_table_from_entity(ArtistEntity))
            .await?;
        manager
            .exec_stmt(schema.create_table_from_entity(ReleaseEntity))
            .await?;
        manager
            .exec_stmt(schema.create_table_from_entity(ArtistReleaseEntity))
            .await?;
        Ok(())
    }
}
