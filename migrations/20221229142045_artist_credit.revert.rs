use sqlx::{Executor, Sqlite};
use sqlx_migrate::prelude::*;

pub async fn revert_artist_group(
    mut ctx: MigrationContext<'_, Sqlite>,
) -> Result<(), MigrationError> {
    Ok(())
}
