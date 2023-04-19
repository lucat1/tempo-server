use sea_orm::DbConn;
use serde::Deserialize;

#[derive(Clone)]
pub struct AppState(pub DbConn);

#[derive(Deserialize)]
pub struct QueryParameters {
    pub include: Option<String>,
    pub sort: Option<String>,
    #[serde(rename = "filter[year]")]
    pub filter_year: Option<String>,
}
