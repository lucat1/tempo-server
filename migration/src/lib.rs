use sea_orm_migration::prelude::*;

mod m20220101_000001_init;
mod m20230416_000001_image;
mod m20230511_000001_artist_description;
mod m20230513_000001_artist_url;
mod m20230525_000001_user;
mod m20230602_000001_scrobble;
mod m20230602_000002_release_date;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_init::Migration),
            Box::new(m20230416_000001_image::Migration),
            Box::new(m20230511_000001_artist_description::Migration),
            Box::new(m20230513_000001_artist_url::Migration),
            Box::new(m20230525_000001_user::Migration),
            Box::new(m20230602_000001_scrobble::Migration),
            Box::new(m20230602_000002_release_date::Migration),
        ]
    }
}
