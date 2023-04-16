use sea_orm::DbConn;

#[derive(Clone)]
pub struct AppState(pub DbConn);
