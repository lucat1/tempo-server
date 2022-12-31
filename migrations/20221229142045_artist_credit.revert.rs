use sqlx::Sqlite;
use sqlx_migrate::prelude::*;

pub async fn revert_artist_credit(
    _ctx: MigrationContext<'_, Sqlite>,
) -> Result<(), MigrationError> {
    Ok(())
}
