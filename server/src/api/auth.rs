use axum::{
    async_trait,
    extract::FromRequestParts,
    headers::authorization::{Authorization, Bearer},
    http::Request,
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use common::auth::authenticate;
use eyre::Result;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Add};
use time::{Duration, OffsetDateTime};

use crate::api::{
    documents::{AuthAttributes, AuthRelation, AuthResource, ResourceType, Token},
    extract::{Json, TypedHeader},
    jsonapi::{
        Document, DocumentData, Error, Related, Relation, Relationship, Resource,
        ResourceIdentifier,
    },
};
use base::setting::{get_settings, Settings};

use super::documents::Included;

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
        let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
            .await?
            .inner();
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
    auth_header: TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Document<AuthResource, Included>>, Error> {
    let auth = auth_header.inner();
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
    auth_header: TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Document<AuthResource, Included>>, Error> {
    let auth = auth_header.inner();
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
