use sea_orm::entity::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "artists_url")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub artist_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub r#type: UrlType,
    pub url: String,
}

#[derive(
    Serialize, Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, PartialOrd, Ord, Hash,
)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum UrlType {
    #[sea_orm(string_value = "biography")]
    #[serde(rename = "biography")]
    Biography,
    #[sea_orm(string_value = "homepage")]
    #[serde(rename = "homepage")]
    Homepage,
    #[sea_orm(string_value = "lastfm")]
    #[serde(rename = "lastfm")]
    LastFM,
    #[sea_orm(string_value = "discogs")]
    #[serde(rename = "discogs")]
    Discogs,
    #[sea_orm(string_value = "songkick")]
    #[serde(rename = "songkick")]
    SongKick,
    #[sea_orm(string_value = "allmusic")]
    #[serde(rename = "allmusic")]
    AllMusic,
    #[sea_orm(string_value = "soundcloud")]
    #[serde(rename = "soundcloud")]
    SoundCloud,
    #[sea_orm(string_value = "spotify")]
    #[serde(rename = "spotify")]
    Spotify,
    #[sea_orm(string_value = "deezer")]
    #[serde(rename = "deezer")]
    Deezer,
    #[sea_orm(string_value = "tidal")]
    #[serde(rename = "tidal")]
    Tidal,
    #[sea_orm(string_value = "wikidata")]
    #[serde(rename = "wikidata")]
    Wikidata,
    #[sea_orm(string_value = "youtube")]
    #[serde(rename = "youtube")]
    Youtube,
    #[sea_orm(string_value = "twitter")]
    #[serde(rename = "twitter")]
    Twitter,
    #[sea_orm(string_value = "facebook")]
    #[serde(rename = "facebook")]
    Facebook,
    #[sea_orm(string_value = "instagram")]
    #[serde(rename = "instagram")]
    Instagram,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::Id"
    )]
    Artist,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
