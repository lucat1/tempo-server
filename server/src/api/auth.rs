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
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Add};
use time::{Duration, OffsetDateTime};

use crate::api::{
    documents::{AuthAttributes, AuthRelation, Token},
    extract::{Json, TypedHeader},
    jsonapi::{
        AuthResource, Document, DocumentData, Error, Related, Relation, Relationship, Resource,
        ResourceIdentifier, ResourceType,
    },
};
use base::setting::get_settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
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
}

fn check_token(token: &str) -> Result<TokenData<Claims>, Error> {
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while checking user authentication".to_owned(),
        detail: Some(e.into()),
    })?;
    let claims = decode::<Claims>(
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
                data: Relation::Single(Related::User(ResourceIdentifier {
                    r#type: ResourceType::User,
                    id: username,
                    meta: None,
                })),
            },
        )]
        .into(),
        meta: HashMap::new(),
    }
}

pub async fn auth(
    auth_header: TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Document<AuthResource>>, Error> {
    let auth = auth_header.inner();
    match check_token(auth.token()) {
        Ok(token_data) => Ok(Json::new(Document {
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
        })),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
pub struct LoginData {
    username: String,
    password: String,
}

pub async fn login(
    json_login_data: Json<LoginData>,
) -> Result<Json<Document<AuthResource>>, Error> {
    let login_data = json_login_data.inner();
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

    let token_expiry = OffsetDateTime::now_utc().add(Duration::days(7));
    let claims = Claims {
        username: user.username.to_owned(),
        exp: token_expiry.unix_timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.auth.jwt_secret.as_ref()),
    )
    .map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not sign JWT".to_owned(),
        detail: Some(e.into()),
    })?;

    let refresh_expiry = OffsetDateTime::now_utc().add(Duration::days(30));
    let refresh_claims = RefreshClaims {
        username: user.username,
        exp: refresh_expiry.unix_timestamp() as usize,
    };
    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(settings.auth.jwt_secret.as_ref()),
    )
    .map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not sign refresh JWT".to_owned(),
        detail: Some(e.into()),
    })?;

    Ok(Json::new(Document {
        data: DocumentData::Single(auth_resource(
            Token {
                value: token,
                expires_at: token_expiry,
            },
            Some(Token {
                value: refresh_token,
                expires_at: refresh_expiry,
            }),
            claims.username,
        )),
        included: vec![],
        links: HashMap::new(),
    }))
}
