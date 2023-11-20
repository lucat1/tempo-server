use sea_orm::{entity::prelude::*, TryFromU64, TryGetableFromJson};
use sea_query::ValueType;
use serde::{Deserialize, Serialize};
use serde_enum_str::Deserialize_enum_str;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "update")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub subject: Subject,

    pub time: time::OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    id: Identifier,
    r#type: UpdateType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Identifier {
    Uuid(uuid::Uuid),
    String(String),
}

impl TryFrom<String> for Subject {
    type Error = sea_query::ValueTypeErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts_iter = value.split(":").take(2);
        let raw_type = parts_iter.next().ok_or(sea_query::ValueTypeErr)?;
        let r#type =
            serde_json::from_str::<UpdateType>(raw_type).map_err(|_| sea_query::ValueTypeErr)?;
        let raw_id = parts_iter.next().ok_or(sea_query::ValueTypeErr)?;
        let id = serde_json::from_str::<Identifier>(raw_id).map_err(|_| sea_query::ValueTypeErr)?;
        Ok(Subject { id, r#type })
    }
}

impl From<Subject> for String {
    fn from(value: Subject) -> Self {
        let id = match value.id {
            Identifier::String(s) => s,
            Identifier::Uuid(u) => u.to_string(),
        };
        format!("{}:{}", value.r#type.to_string(), id)
    }
}

impl From<Subject> for sea_orm::Value {
    fn from(value: Subject) -> Self {
        Self::String(Some(Box::new(value.into())))
    }
}

impl TryFromU64 for Subject {
    fn try_from_u64(n: u64) -> Result<Self, DbErr> {
        Err(DbErr::ConvertFromU64("Subject"))
    }
}

impl TryGetableFromJson for Subject {}

impl ValueType for Subject {
    fn try_from(v: Value) -> Result<Self, sea_query::ValueTypeErr> {
        match v {
            Value::String(Some(x)) => (*x).try_into(),
            _ => Err(sea_query::ValueTypeErr),
        }
    }

    fn type_name() -> String {
        "Subject".to_string()
    }

    fn array_type() -> sea_query::ArrayType {
        sea_query::ArrayType::String
    }

    fn column_type() -> sea_query::ColumnType {
        sea_query::ColumnType::String(None)
    }
}

#[derive(
    Deserialize_enum_str,
    Serialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    PartialOrd,
    Ord,
    Hash,
)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum UpdateType {
    #[sea_orm(string_value = "artist_description")]
    #[serde(rename = "artist_description")]
    ArtistDescription,

    #[sea_orm(string_value = "artist_url")]
    #[serde(rename = "artist_url")]
    ArtistUrl,

    #[sea_orm(string_value = "lastfm_artist_image")]
    #[serde(rename = "lastfm_artist_image")]
    LastFMArtistImage,

    #[sea_orm(string_value = "index_search")]
    #[serde(rename = "index_search")]
    IndexSearch,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
