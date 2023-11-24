use axum::{
    extract::State,
    headers::authorization::{Authorization, Bearer},
    http::Request,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{encode, errors::Error as JWTError, EncodingKey, Header};
use ldap3::{LdapConnAsync, LdapError, Scope, SearchEntry};
use password_hash::{Error as PasswordHashError, PasswordHash, PasswordVerifier};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Add};
use strfmt::{strfmt, FmtError};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

use argon2::Argon2;
use pbkdf2::Pbkdf2;
use scrypt::Scrypt;

use super::documents::Included;
use crate::api::{
    documents::{AuthAttributes, AuthRelation, AuthResource, ResourceType, Token},
    extract::{check_token, Claims, ClaimsSubject, Json, TypedHeader},
    jsonapi::{
        Document, DocumentData, Related, Relation, Relationship, Resource, ResourceIdentifier,
    },
    AppState, Error,
};
use base::setting::{get_settings, AuthMethod, Settings, SettingsError};

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Database error: {0}")]
    Database(#[from] DbErr),
    #[error("Could not get the settings: {0}")]
    Settings(#[from] SettingsError),
    #[error("Error during jwt serialization: {0}")]
    Jwt(#[from] JWTError),

    #[error("No matching user exists or no auth methods are available")]
    NoCandidate,
    #[error("No user with matching username found")]
    NoMatchingUser,
    #[error("Invalid password hash: {0}")]
    InvalidPasswordHash(#[from] PasswordHashError),

    #[error("Error while formatting LDAP filter: {0}")]
    LdapFilterFormat(#[from] FmtError),

    #[error("Error in LDAP query: {0}")]
    LdapError(#[from] LdapError),
    #[error("LDAP user not found after bind")]
    LdapUserNotFound,
    #[error("LDAP entity is missing the required fields")]
    LdapMissingFIeld,
}

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
) -> Result<UserFields, AuthError> {
    let user = settings
        .auth
        .users
        .iter()
        .find(|u| u.username == username)
        .ok_or(AuthError::NoMatchingUser)?
        .to_owned();
    let password_hash =
        PasswordHash::new(user.password.as_str()).map_err(AuthError::InvalidPasswordHash)?;
    let algs: &[&dyn PasswordVerifier] = &[&Argon2::default(), &Pbkdf2, &Scrypt];
    password_hash.verify_password(algs, password)?;

    Ok(UserFields {
        username: user.username,
        first_name: user.first_name,
        last_name: user.last_name,
    })
}

async fn try_ldap_login(
    settings: &Settings,
    username: &str,
    password: &str,
) -> Result<UserFields, AuthError> {
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

    let first_search_result = rs.into_iter().next().ok_or(AuthError::LdapUserNotFound)?;
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
        .ok_or(AuthError::LdapMissingFIeld)?;
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
) -> Result<entity::User, AuthError>
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

pub async fn authenticate<C>(
    db: &C,
    username: &str,
    password: &str,
) -> Result<entity::User, AuthError>
where
    C: ConnectionTrait,
{
    let settings = get_settings()?;

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
    Err(AuthError::NoCandidate)
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshClaims {
    pub username: String,
    pub exp: usize,
    pub sub: ClaimsSubject,
}

pub async fn auth_middleware<B>(
    _claims: Claims,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, Error> {
    let response = next.run(request).await;
    Ok(response)
}

fn auth_resource(token: Token, refresh: Option<Token>, username: String) -> AuthResource {
    Resource {
        r#type: ResourceType::Auth,
        id: username.to_owned(),
        attributes: AuthAttributes { token, refresh },
        relationships: [(
            AuthRelation::User,
            Relationship {
                data: Relation::Single(Related::String(ResourceIdentifier {
                    r#type: ResourceType::User,
                    id: username,
                    meta: None,
                })),
            },
        )]
        .into(),
        meta: None,
    }
}

pub async fn auth(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Document<AuthResource, Included>>, Error> {
    let token_data = check_token::<Claims>(auth.token())?;
    Ok(Json(Document {
        data: DocumentData::Single(auth_resource(
            Token {
                value: auth.token().to_string(),
                expires_at: OffsetDateTime::now_utc(),
            },
            None,
            token_data.claims.username,
        )),
        included: vec![],
        links: HashMap::new(),
    }))
}

#[derive(Deserialize)]
pub struct LoginData {
    username: String,
    password: String,
}

fn token_pair(settings: &Settings, username: &str) -> Result<(Token, Token), AuthError> {
    let token_expiry = OffsetDateTime::now_utc().add(Duration::days(7));
    let claims = Claims {
        username: username.to_owned(),
        exp: token_expiry.unix_timestamp() as usize,
        sub: ClaimsSubject::Token,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.auth.jwt_secret.as_ref()),
    )?;

    let refresh_expiry = OffsetDateTime::now_utc().add(Duration::days(30));
    let refresh_claims = RefreshClaims {
        username: username.to_owned(),
        exp: refresh_expiry.unix_timestamp() as usize,
        sub: ClaimsSubject::Refresh,
    };
    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(settings.auth.jwt_secret.as_ref()),
    )?;
    Ok((
        Token {
            value: token,
            expires_at: token_expiry,
        },
        Token {
            value: refresh_token,
            expires_at: refresh_expiry,
        },
    ))
}

pub async fn login(
    State(AppState(db)): State<AppState>,
    Json(login_data): Json<LoginData>,
) -> Result<Json<Document<AuthResource, Included>>, Error> {
    let settings = get_settings()?;
    let user = authenticate(
        &db,
        login_data.username.as_str(),
        login_data.password.as_str(),
    )
    .await
    .map_err(|_| Error::Unauthorized(Some("Authentication failed".to_string())))?;
    let (token, refresh_token) = token_pair(settings, user.username.as_str())?;

    Ok(Json(Document {
        data: DocumentData::Single(auth_resource(token, Some(refresh_token), user.username)),
        included: vec![],
        links: HashMap::new(),
    }))
}

pub async fn refresh(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Document<AuthResource, Included>>, Error> {
    let token_data = check_token::<Claims>(auth.token())?;
    if token_data.claims.sub != ClaimsSubject::Refresh {
        return Err(Error::BadRequest(Some("Invalid refresh token".to_owned())));
    }
    let settings = get_settings()?;
    let (token, refresh_token) = token_pair(settings, token_data.claims.username.as_str())?;
    Ok(Json(Document {
        data: DocumentData::Single(auth_resource(
            token,
            Some(refresh_token),
            token_data.claims.username,
        )),
        included: vec![],
        links: HashMap::new(),
    }))
}
