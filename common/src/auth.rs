use eyre::{eyre, Result};
use ldap3::{LdapConnAsync, Scope, SearchEntry};
use password_hash::{PasswordHash, PasswordVerifier};
use strfmt::strfmt;

use argon2::Argon2;
use pbkdf2::Pbkdf2;
use scrypt::Scrypt;

use base::{
    database::get_database,
    setting::{get_settings, AuthMethod, Settings},
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, IntoActiveModel};

struct UserFields {
    username: String,
    first_name: Option<String>,
    last_name: Option<String>,
}

async fn try_local_login(
    settings: &Settings,
    username: &str,
    password: &str,
) -> Result<UserFields> {
    let user = settings
        .auth
        .users
        .iter()
        .find(|u| u.username == username)
        .ok_or(eyre!("No user with matching username found"))?
        .to_owned();
    let password_hash = PasswordHash::new(user.password.as_str())
        .map_err(|e| eyre!("Invalid password hash: {}", e))?;
    let algs: &[&dyn PasswordVerifier] = &[&Argon2::default(), &Pbkdf2, &Scrypt];
    password_hash.verify_password(algs, password)?;

    Ok(UserFields {
        username: user.username,
        first_name: user.first_name,
        last_name: user.last_name,
    })
}

async fn try_ldap_login(settings: &Settings, username: &str, password: &str) -> Result<UserFields> {
    let (conn, mut ldap) = LdapConnAsync::new("ldap://localhost:2389").await?;
    ldap3::drive!(conn);
    let vars = [("username".to_string(), username.to_string())].into();
    let bind_dn = strfmt(settings.auth.ldap_user_dn.as_str(), &vars)?;
    tracing::trace!(%bind_dn, "Binding on LDAP");
    ldap.simple_bind(bind_dn.as_str(), password).await?;
    let (rs, _res) = ldap
        .search(
            bind_dn.as_str(),
            Scope::Base,
            "",
            vec![
                settings.auth.ldap_attr_map.username.as_str(),
                settings.auth.ldap_attr_map.first_name.as_str(),
                settings.auth.ldap_attr_map.last_name.as_str(),
            ],
        )
        .await?
        .success()?;
    let user = rs
        .into_iter()
        .next()
        .ok_or(eyre!("Expected to find the user after a successful bind"))?;
    tracing::info!(entity = ?SearchEntry::construct(user), "user entity");
    ldap.unbind().await?;
    Ok(UserFields {
        username: "".to_string(),
        first_name: None,
        last_name: None,
    })
}

async fn get_or_populate<C>(
    db: &C,
    provider: AuthMethod,
    fields: UserFields,
) -> Result<entity::User>
where
    C: ConnectionTrait,
{
    let user = entity::UserEntity::find_by_id(fields.username.to_owned())
        .one(db)
        .await?;
    if let Some(user) = user {
        Ok(user)
    } else {
        let user = entity::User {
            username: fields.username,
            provider: provider.into(),
            first_name: fields.first_name,
            last_name: fields.last_name,
        };
        let user = user.into_active_model().insert(db).await?;
        Ok(user)
    }
}

pub async fn authenticate(username: &str, password: &str) -> Result<entity::User> {
    let settings = get_settings()?;
    let db = get_database()?;

    let mut methods = settings.auth.priority.iter();
    while let Some(method) = methods.next() {
        let result = match method {
            AuthMethod::Local => try_local_login(&settings, username, password).await,
            AuthMethod::LDAP => try_ldap_login(&settings, username, password).await,
        };
        match result {
            Ok(fields) => return get_or_populate(db, *method, fields).await,
            Err(e) => tracing::trace!(?method, %e, "Login attempt failed"),
        }
    }
    Err(eyre!(
        "No matching user exists or no auth methods are available"
    ))
}
