use sea_orm::prelude::*;
use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use time::Date;
use uuid::Uuid;

use entity::{ReleaseColumn, ReleaseEntity};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Debug, Clone, Copy, DeriveColumn)]
enum OldColumns {
    Date,
    OriginalDate,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let conn = manager.get_connection();
        let maybe_dates = conn
            .query_all(Statement::from_string(
                backend,
                "SELECT id, date, original_date FROM release".to_string(),
            ))
            .await;
        let mut map: HashMap<Uuid, (Option<Date>, Option<Date>)> = HashMap::new();
        if let Ok(dates) = maybe_dates {
            for result in dates.iter() {
                let id: Uuid = result.try_get_by_index(0)?;
                let date: Option<Date> = result.try_get_by_index(1)?;
                let original_date: Option<Date> = result.try_get_by_index(2)?;
                map.insert(id, (date, original_date));
            }

            manager
                .alter_table(
                    sea_query::Table::alter()
                        .table(ReleaseEntity)
                        .drop_column(OldColumns::Date)
                        .drop_column(OldColumns::OriginalDate)
                        .to_owned(),
                )
                .await?;
            let mut _alter = sea_query::Table::alter();
            let mut alter = &mut _alter;
            let columns = [
                ReleaseColumn::Year,
                ReleaseColumn::Month,
                ReleaseColumn::Day,
                ReleaseColumn::OriginalYear,
                ReleaseColumn::OriginalMonth,
                ReleaseColumn::OriginalDay,
            ];
            for col in columns.into_iter() {
                alter =
                    alter.add_column(&mut sea_orm_migration::prelude::ColumnDef::new_with_type(
                        col,
                        col.def().get_column_type().clone(),
                    ));
            }
            manager
                .alter_table(alter.table(ReleaseEntity).to_owned())
                .await?;

            for (id, (date, original_date)) in map.into_iter() {
                let mut values = Vec::new();
                if let Some(date) = date {
                    values.push((ReleaseColumn::Year, date.year().into()));
                    values.push((ReleaseColumn::Month, (date.month() as i16).into()));
                    values.push((ReleaseColumn::Day, date.day().into()));
                }
                if let Some(original_date) = original_date {
                    values.push((ReleaseColumn::OriginalYear, original_date.year().into()));
                    values.push((
                        ReleaseColumn::OriginalMonth,
                        (original_date.month() as i16).into(),
                    ));
                    values.push((ReleaseColumn::OriginalDay, original_date.day().into()));
                }
                if !values.is_empty() {
                    let builder = sea_query::Query::update()
                        .table(ReleaseEntity)
                        .values(values)
                        .and_where(Expr::col(ReleaseColumn::Id).eq(id))
                        .to_owned();

                    let (sql, values) = match backend {
                        DatabaseBackend::MySql => builder.build(MysqlQueryBuilder),
                        DatabaseBackend::Postgres => builder.build(PostgresQueryBuilder),
                        DatabaseBackend::Sqlite => builder.build(SqliteQueryBuilder),
                    };
                    conn.execute(Statement::from_sql_and_values(
                        backend,
                        sql.as_str(),
                        values,
                    ))
                    .await?;
                }
            }
        }
        // Migration is not needed as this is a new instance with no old release entries
        // the query would error as no columns named `date` and `original_date` exist.
        Ok(())
    }
}
