use std::{collections::HashMap, ops::Add};

use axum::{
    extract::TypedHeader,
    headers::authorization::{Authorization, Bearer},
    http::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
use chrono::{prelude::*, Duration};
use common::auth::authenticate;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};

use crate::{
    documents::{AuthAttributes, AuthRelation, Token},
    jsonapi::{
        AuthResource, Document, DocumentData, Error, Related, Relation, Relationship, Resource,
        ResourceIdentifier, ResourceType,
    },
};
use base::setting::get_settings;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub username: String,
    pub exp: usize,
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
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, Error> {
    let _ = check_token(auth.token())?;
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
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Document<AuthResource>>, Error> {
    match check_token(auth.token()) {
        Ok(token_data) => Ok(Json(Document {
            data: DocumentData::Single(auth_resource(
                Token {
                    value: auth.token().to_string(),
                    expires_at: Utc::now(),
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
    Json(login_data): Json<LoginData>,
) -> Result<Json<Document<AuthResource>>, Error> {
    // TODO: check credentials against a user store
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

    let token_expiry = Utc::now().add(Duration::days(7));
    let claims = Claims {
        username: user.username.to_owned(),
        exp: token_expiry.timestamp() as usize,
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

    let refresh_expiry = Utc::now().add(Duration::days(30));
    let refresh_claims = RefreshClaims {
        username: user.username.to_owned(),
        exp: refresh_expiry.timestamp() as usize,
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

    Ok(Json(Document {
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
