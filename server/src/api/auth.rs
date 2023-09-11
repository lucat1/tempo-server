use axum::{
    async_trait,
    extract::FromRequestParts,
    headers::authorization::{Authorization, Bearer},
    http::Request,
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use eyre::{eyre, Result};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use ldap3::{LdapConnAsync, Scope, SearchEntry};
use password_hash::{PasswordHash, PasswordVerifier};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Add};
use strfmt::strfmt;
use time::{Duration, OffsetDateTime};

use argon2::Argon2;
use pbkdf2::Pbkdf2;
use scrypt::Scrypt;

use super::documents::Included;
use crate::api::{
    documents::{AuthAttributes, AuthRelation, AuthResource, ResourceType, Token},
    extract::{Json, TypedHeader},
    jsonapi::{
        Document, DocumentData, Error, Related, Relation, Relationship, Resource,
        ResourceIdentifier,
    },
};
use base::{
    database::get_database,
    setting::{get_settings, AuthMethod, Settings},
};

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClaimsSubject {
    Token,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
    pub sub: ClaimsSubject,
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(header) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
        check_token(header.token()).map(|td| td.claims)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshClaims {
    pub username: String,
    pub exp: usize,
    pub sub: ClaimsSubject,
}

fn check_token<T>(token: &str) -> Result<TokenData<T>, Error>
where
    T: for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while checking user authentication".to_owned(),
        detail: Some(e.into()),
    })?;
    let claims = decode::<T>(
        token,
        &DecodingKey::from_secret(settings.auth.jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    );
    match claims {
        Ok(token_data) => {
            tracing::trace!(?token_data, "User for request");
            Ok(token_data)
        }
        Err(e) => Err(Error {
            status: StatusCode::UNAUTHORIZED,
            title: "Invalid authentication token".to_owned(),
            detail: Some(e.into()),
        }),
    }
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

fn token_pair(settings: &Settings, username: &str) -> Result<(Token, Token)> {
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
    Json(login_data): Json<LoginData>,
) -> Result<Json<Document<AuthResource, Included>>, Error> {
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while checking user authentication".to_owned(),
        detail: Some(e.into()),
    })?;
    let user = authenticate(login_data.username.as_str(), login_data.password.as_str())
        .await
        .map_err(|e| Error {
            status: StatusCode::UNAUTHORIZED,
            title: "Authentication failed".to_string(),
            detail: Some(e.into()),
        })?;
    let (token, refresh_token) =
        token_pair(settings, user.username.as_str()).map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not sign JWT".to_owned(),
            detail: Some(e.into()),
        })?;

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
        return Err(Error {
            status: StatusCode::BAD_REQUEST,
            title: "Invalid refresh token".to_owned(),
            detail: None,
        });
    }
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while checking user authentication".to_owned(),
        detail: Some(e.into()),
    })?;
    let (token, refresh_token) = token_pair(settings, token_data.claims.username.as_str())
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not sign JWT".to_owned(),
            detail: Some(e.into()),
        })?;
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
