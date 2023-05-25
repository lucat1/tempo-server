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
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel};

#[derive(Debug, Clone)]
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
    let (conn, mut ldap) = LdapConnAsync::new(settings.auth.ldap.uri.as_str()).await?;
    ldap3::drive!(conn);
    tracing::trace!(admin_dn = %settings.auth.ldap.admin_dn, "Binding as LDAP Admin user");
    ldap.simple_bind(
        settings.auth.ldap.admin_dn.as_str(),
        settings.auth.ldap.admin_pw.as_str(),
    )
    .await?
    .success()?;

    let vars = [("username".to_string(), username.to_string())].into();
    let filter = strfmt(settings.auth.ldap.user_filter.as_str(), &vars)?;
    tracing::trace!(%filter, base_dn = %settings.auth.ldap.base_dn,"Searching for user attributes");
    let (rs, _res) = ldap
        .search(
            settings.auth.ldap.base_dn.as_str(),
            Scope::Subtree,
            filter.as_str(),
            vec![
                settings.auth.ldap.attr_map.username.as_str(),
                settings.auth.ldap.attr_map.first_name.as_str(),
                settings.auth.ldap.attr_map.last_name.as_str(),
            ],
        )
        .await?
        .success()?;

    let first_search_result = rs
        .into_iter()
        .next()
        .ok_or(eyre!("Expected to find the user after a successful bind"))?;
    let entity = SearchEntry::construct(first_search_result);
    tracing::info!(entity = ?entity, "Found user entity");
    // ldap.unbind().await?;

    tracing::trace!(bind_dn = entity.dn, "Binding as LDAP authenticating user");
    ldap.simple_bind(entity.dn.as_str(), password)
        .await?
        .success()?;
    ldap.unbind().await?;

    tracing::trace!(bind_dn = entity.dn, "Successfully authenticated");
    let empty_vec = Vec::new();
    let username = entity
        .attrs
        .get(&settings.auth.ldap.attr_map.username)
        .unwrap_or(&empty_vec)
        .first()
        .ok_or(eyre!(
            "LDAP user entity is missing the mapped field for username"
        ))?;
    let first_name = entity
        .attrs
        .get(&settings.auth.ldap.attr_map.first_name)
        .unwrap_or(&empty_vec)
        .first();
    let last_name = entity
        .attrs
        .get(&settings.auth.ldap.attr_map.last_name)
        .unwrap_or(&empty_vec)
        .first();
    Ok(UserFields {
        username: username.to_owned(),
        first_name: first_name.map(|s| s.to_owned()),
        last_name: last_name.map(|s| s.to_owned()),
    })
}

async fn update_or_create<C>(
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
    if let Some(mut user) = user {
        if user.first_name != fields.first_name || user.last_name != fields.last_name {
            tracing::trace!(?fields, "Updating user with new field values");
            let mut active_user = user.into_active_model();
            active_user.first_name = ActiveValue::Set(fields.first_name);
            active_user.last_name = ActiveValue::Set(fields.last_name);
            active_user.clone().update(db).await?;
            user = active_user.try_into()?;
        }
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

    for method in settings.auth.priority.iter() {
        let result = match method {
            AuthMethod::Local => try_local_login(settings, username, password).await,
            AuthMethod::Ldap => try_ldap_login(settings, username, password).await,
        };
        match result {
            Ok(fields) => return update_or_create(db, *method, fields).await,
            Err(e) => tracing::trace!(?method, %e, "Login attempt failed"),
        }
    }
    Err(eyre!(
        "No matching user exists or no auth methods are available"
    ))
}
