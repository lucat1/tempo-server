use sea_orm::prelude::*;
use sea_orm::{ColumnTrait, DatabaseBackend, Schema, Statement};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use entity::{
    conflict::{GENRE_CONFLICT, GENRE_RELEASE_CONFLICT, GENRE_TRACK_CONFLICT},
    GenreColumn, GenreEntity, GenreReleaseColumn, GenreReleaseEntity, GenreTrackColumn,
    GenreTrackEntity, ImportColumn, ImportEntity, ReleaseEntity, TrackEntity,
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
    ($conn: expr, $backend: expr, $manager: expr, $table: expr, $entity: expr) => {{
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

        let mut all_genres: Vec<String> = map.iter().flat_map(|(_, v)| v.to_owned()).collect();
        all_genres.sort_unstable();
        all_genres.dedup();
        let all_genres: Vec<[SimpleExpr; 3]> = all_genres
            .into_iter()
            .map(|g| [sha256::digest(&g).into(), g.to_owned().into(), g.into()])
            .collect();
        let mut builder = sea_query::Query::insert()
            .into_table(GenreEntity)
            .columns([
                GenreColumn::Id,
                GenreColumn::Name,
                GenreColumn::Disambiguation,
            ])
            .on_conflict(GENRE_CONFLICT.to_owned())
            .to_owned();
        for genre in all_genres.into_iter() {
            builder = builder.values_panic(genre).to_owned();
        }
        execute!($conn, $backend, builder);
        map
    }};
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let schema = Schema::new(backend);
        let mut binding = Table::alter();

        let table = binding
            .table(ImportEntity)
            .add_column_if_not_exists(&mut sea_query::table::ColumnDef::new_with_type(
                ImportColumn::Genres,
                ImportColumn::Genres.def().get_column_type().clone(),
            ))
            .add_column_if_not_exists(&mut sea_query::table::ColumnDef::new_with_type(
                ImportColumn::TrackGenres,
                ImportColumn::TrackGenres.def().get_column_type().clone(),
            ))
            .add_column_if_not_exists(&mut sea_query::table::ColumnDef::new_with_type(
                ImportColumn::ReleaseGenres,
                ImportColumn::ReleaseGenres.def().get_column_type().clone(),
            ));
        manager.alter_table(table.to_owned()).await?;

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
        if !has_genres.is_empty() {
            let map = relationalize!(conn, backend, manager, "release", ReleaseEntity);
            for (id, genres) in map.into_iter() {
                if !genres.is_empty() {
                    let values: Vec<[SimpleExpr; 2]> = genres
                        .into_iter()
                        .map(|g| [sha256::digest(g).into(), id.into()])
                        .collect();
                    let mut builder = sea_query::Query::insert()
                        .into_table(GenreReleaseEntity)
                        .columns([GenreReleaseColumn::GenreId, GenreReleaseColumn::ReleaseId])
                        .on_conflict(GENRE_RELEASE_CONFLICT.to_owned())
                        .to_owned();
                    for value in values.into_iter() {
                        builder = builder.values_panic(value).to_owned();
                    }
                    execute!(conn, backend, builder);
                }
            }
            let map = relationalize!(conn, backend, manager, "track", TrackEntity);
            for (id, genres) in map.into_iter() {
                if !genres.is_empty() {
                    let genres_len = genres.len();
                    let values: Vec<[SimpleExpr; 3]> = genres
                        .into_iter()
                        .enumerate()
                        .map(|(i, g)| {
                            [
                                sha256::digest(g).into(),
                                id.into(),
                                ((genres_len - i) as i32).into(),
                            ]
                        })
                        .collect();
                    let mut builder = sea_query::Query::insert()
                        .into_table(GenreTrackEntity)
                        .columns([
                            GenreTrackColumn::GenreId,
                            GenreTrackColumn::TrackId,
                            GenreTrackColumn::Cnt,
                        ])
                        .on_conflict(GENRE_TRACK_CONFLICT.to_owned())
                        .to_owned();
                    for value in values.into_iter() {
                        builder = builder.values_panic(value).to_owned();
                    }
                    execute!(conn, backend, builder);
                }
            }
        }
        Ok(())
    }
}
