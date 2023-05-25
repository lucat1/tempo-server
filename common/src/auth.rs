use eyre::{eyre, Result};
use password_hash::{PasswordHash, PasswordVerifier};

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
        .filter(|u| u.username == username)
        .next()
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

async fn try_ldap_login(
    _settings: &Settings,
    _username: &str,
    _password: &str,
) -> Result<UserFields> {
    unimplemented!()
}

async fn get_or_populate<C>(db: &C, fields: UserFields) -> Result<entity::User>
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
            Ok(fields) => return get_or_populate(db, fields).await,
            Err(e) => tracing::trace!(?method, %e, "Login attempt failed"),
        }
    }
    Err(eyre!(
        "No matching user exists or no auth methods are available"
    ))
}
