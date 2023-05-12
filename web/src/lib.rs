pub mod fetch;
pub mod tasks;

use sea_orm::DbConn;
use serde::Deserialize;

pub use tasks::get_queue;

#[derive(Clone)]
pub struct AppState(pub DbConn);

#[derive(Deserialize)]
pub struct QueryParameters {
    pub include: Option<String>,
    pub sort: Option<String>,
    #[serde(rename = "filter[year]")]
    pub filter_year: Option<String>,
}
