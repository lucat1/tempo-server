use std::hash::Hash;

use base::setting::AuthMethod;
use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i8", db_type = "Integer")]
pub enum AuthProvider {
    #[sea_orm(num_value = 0)]
    Local,
    #[sea_orm(num_value = 1)]
    Ldap,
}

#[derive(Serialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub username: String,
    pub provider: AuthProvider,

    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

impl From<AuthMethod> for AuthProvider {
    fn from(value: AuthMethod) -> Self {
        match value {
            AuthMethod::Local => AuthProvider::Local,
            AuthMethod::Ldap => AuthProvider::Ldap,
        }
    }
}

impl From<AuthProvider> for AuthMethod {
    fn from(value: AuthProvider) -> Self {
        match value {
            AuthProvider::Local => AuthMethod::Local,
            AuthProvider::Ldap => AuthMethod::Ldap,
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Hash for Column {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.to_string().eq(&other.to_string())
    }
}

impl Eq for Column {}

impl TryFrom<String> for Column {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "username" => Ok(Column::Username),
            "first_name" => Ok(Column::FirstName),
            "last_name" => Ok(Column::LastName),
            &_ => Err("Invalid column name".to_owned()),
        }
    }
}
