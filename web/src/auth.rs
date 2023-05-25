use axum::{
    extract::TypedHeader,
    headers::authorization::{Authorization, Bearer},
    http::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::jsonapi::Error;
use base::setting::get_settings;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub id: Uuid,
}

pub async fn auth<B>(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, Error> {
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while checking user authentication".to_owned(),
        detail: Some(e.into()),
    })?;
    let claims = decode::<Claims>(
        auth.token(),
        &DecodingKey::from_secret(settings.keys.jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    );
    match claims {
        Ok(token_data) => {
            tracing::trace!(?token_data, "User for request");
            let response = next.run(request).await;
            Ok(response)
        }
        Err(e) => Err(Error {
            status: StatusCode::UNAUTHORIZED,
            title: "Invalid authentication token".to_owned(),
            detail: Some(e.into()),
        }),
    }
}
