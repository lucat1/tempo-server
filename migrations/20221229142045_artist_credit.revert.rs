use sqlx::Sqlite;
use sqlx_migrate::prelude::*;

pub async fn revert_artist_credit(ctx: MigrationContext<'_, Sqlite>) -> Result<(), MigrationError> {
    Ok(())
}
