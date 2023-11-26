use sea_orm::prelude::*;
use sea_orm::{DatabaseBackend, Schema, Statement};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use entity::{
    GenreColumn, GenreEntity, GenreReleaseColumn, GenreReleaseEntity, GenreTrackColumn,
    GenreTrackEntity, ReleaseEntity, TrackEntity,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Debug, Clone, Copy, DeriveColumn)]
enum OldColumn {
    Genres,
}

macro_rules! execute {
    ($conn: expr, $backend: expr, $builder: expr) => {
        let (sql, values) = match $backend {
            DatabaseBackend::MySql => $builder.build(MysqlQueryBuilder),
            DatabaseBackend::Postgres => $builder.build(PostgresQueryBuilder),
            DatabaseBackend::Sqlite => $builder.build(SqliteQueryBuilder),
        };
        $conn
            .execute(Statement::from_sql_and_values(
                $backend,
                sql.as_str(),
                values,
            ))
            .await?;
    };
}

macro_rules! relationalize {
    ($conn: expr, $backend: expr, $manager: expr, $table: expr, $entity: expr, $genre_entity: expr, $genre_entity_column: ident, $genre_entity_field: ident) => {
        let genres = $conn
            .query_all(Statement::from_string(
                $backend,
                format!("SELECT id, genres FROM {}", $table),
            ))
            .await?;
        let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
        for result in genres.iter() {
            let id: Uuid = result.try_get_by_index(0)?;
            let genres: serde_json::Value = result.try_get_by_index(1)?;
            let genres: Vec<String> =
                serde_json::from_value(genres).map_err(|e| DbErr::TryIntoErr {
                    from: "serde_json::Value",
                    into: "Vec<String>",
                    source: Box::new(e),
                })?;
            map.insert(id, genres);
        }
        $manager
            .alter_table(
                sea_query::Table::alter()
                    .table($entity)
                    .drop_column(OldColumn::Genres)
                    .to_owned(),
            )
            .await?;

        let all_genres: Vec<SimpleExpr> = map
            .iter()
            .flat_map(|(_, v)| v)
            .flat_map(|g| [sha256::digest(g).into(), g.into(), g.into()])
            .collect();
        let builder = sea_query::Query::insert()
            .into_table(GenreEntity)
            .columns([
                GenreColumn::Id,
                GenreColumn::Name,
                GenreColumn::Disambiguation,
            ])
            .values_panic(all_genres)
            .to_owned();
        execute!($conn, $backend, builder);

        for (id, genres) in map.into_iter() {
            if !genres.is_empty() {
                let values: Vec<SimpleExpr> = genres
                    .into_iter()
                    .flat_map(|g| [id.into(), g.into()])
                    .collect();
                let builder = sea_query::Query::insert()
                    .into_table($genre_entity)
                    .columns([
                        $genre_entity_column::GenreId,
                        $genre_entity_column::$genre_entity_field,
                    ])
                    .values_panic(values)
                    .to_owned();
                execute!($conn, $backend, builder);
            }
        }
    };
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let schema = Schema::new(backend);
        manager
            .exec_stmt(schema.create_table_from_entity(GenreEntity))
            .await?;
        manager
            .exec_stmt(schema.create_table_from_entity(GenreReleaseEntity))
            .await?;
        manager
            .exec_stmt(schema.create_table_from_entity(GenreTrackEntity))
            .await?;

        let conn = manager.get_connection();
        // We test this on the release table, but it could also be tested on the track table
        let has_genres = conn
            .query_all(Statement::from_string(
                backend,
                "SELECT column_name FROM information_schema.columns WHERE table_name='release' and column_name='genres'".to_string(),
            ))
            .await?;
        // If we do have the genres column then we need to migrate data,
        // otherwise the DB has already been migrated with the new structure.
        if has_genres.is_empty() {
            relationalize!(
                conn,
                backend,
                manager,
                "release",
                ReleaseEntity,
                GenreReleaseEntity,
                GenreReleaseColumn,
                ReleaseId
            );
            relationalize!(
                conn,
                backend,
                manager,
                "track",
                TrackEntity,
                GenreTrackEntity,
                GenreTrackColumn,
                TrackId
            );
        }
        Ok(())
    }
}
