use axum::{
    async_trait,
    extract::{OriginalUri, Query, State},
    http::StatusCode,
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use reqwest::{Method, Request};
use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

use crate::api::{
    auth::Claims,
    documents::{ConnectionAttributes, ConnectionFlow},
    extract::{Json, Path},
    jsonapi::{ConnectionResource, Document, DocumentData, Error, ResourceType},
    AppState,
};
use crate::fetch::lastfm;
use base::setting::{get_settings, Settings};

lazy_static! {
    static ref LASTFM_AUTH_URL: url::Url = url::Url::parse("http://www.last.fm/api/auth").unwrap();
    static ref ID_MAP: Arc<Mutex<HashMap<Uuid, String>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Deserialize)]
pub struct ProviderOptions {
    pub redirect: Option<Url>,
}

#[derive(Deserialize)]
pub struct CallbackOptions {
    pub token: String,
    pub id: Uuid,
    pub redirect: Option<Url>,
}

#[async_trait]
trait ProviderImpl {
    async fn attributes(
        &self,
        settings: &Settings,
        claims: &Claims,
        uri: Uri,
        opts: &ProviderOptions,
    ) -> Result<ConnectionAttributes>;
    async fn callback(
        &self,
        settings: &Settings,
        opts: &CallbackOptions,
    ) -> Result<serde_json::Value>;
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LastFMAuthResponse {
    Success(LastFMAuthResponseSuccess),
    Error(LastFMAuthResponseError),
}

#[derive(Deserialize)]
struct LastFMAuthResponseSuccess {
    session: LastFMSession,
}

#[derive(Deserialize)]
struct LastFMSession {
    key: String,
    name: String,
}

#[derive(Deserialize)]
struct LastFMAuthResponseError {
    code: usize,
    message: String,
}

async fn id(username: &str) -> Uuid {
    let id = Uuid::new_v4();
    ID_MAP.lock().await.insert(id, username.to_owned());
    id
}

async fn username(id: &Uuid) -> Option<String> {
    ID_MAP.lock().await.get(id).cloned()
}

#[async_trait]
impl ProviderImpl for entity::ConnectionProvider {
    async fn attributes(
        &self,
        settings: &Settings,
        claims: &Claims,
        uri: Uri,
        opts: &ProviderOptions,
    ) -> Result<ConnectionAttributes> {
        match self {
            entity::ConnectionProvider::LastFM => {
                if let Some(lastfm) = &settings.connections.lastfm {
                    let mut url = LASTFM_AUTH_URL.clone();
                    let mut cb_url = settings.url.clone();
                    cb_url.set_path(uri.path().to_string().as_str());
                    if let Ok(mut params) = cb_url.path_segments_mut() {
                        params.push("callback");
                    };
                    let id = id(&claims.username).await.to_string();
                    {
                        let cb_url_params = &mut cb_url.query_pairs_mut();
                        cb_url_params.append_pair("id", id.as_str());
                        if let Some(redir) = &opts.redirect {
                            cb_url_params.append_pair("redirect", redir.to_string().as_str());
                        }
                    }
                    url.query_pairs_mut()
                        .append_pair("api_key", lastfm.apikey.as_str())
                        .append_pair("cb", cb_url.to_string().as_str());
                    Ok(ConnectionAttributes {
                        flow: ConnectionFlow::Redirect,
                        url,
                    })
                } else {
                    Err(eyre!("Provider {} not configured", self.to_string()))
                }
            }
        }
    }
    async fn callback(
        &self,
        settings: &Settings,
        opts: &CallbackOptions,
    ) -> Result<serde_json::Value> {
        match self {
            entity::ConnectionProvider::LastFM => {
                if let Some(lastfm) = &settings.connections.lastfm {
                    let mut url = lastfm::LASTFM_BASE_URL.clone();
                    url.query_pairs_mut()
                        .append_pair("method", "auth.getSession")
                        .append_pair("format", "json")
                        .append_pair("api_key", lastfm.apikey.as_str())
                        .append_pair("token", opts.token.as_str());
                    let signature = lastfm::signature(&url, lastfm.shared_secret.as_str());
                    url.query_pairs_mut()
                        .append_pair("api_sig", signature.as_str());
                    let res = lastfm::send_request(Request::new(Method::GET, url)).await?;
                    let raw_data: LastFMAuthResponse = res.json().await.map_err(|e| eyre!(e))?;
                    match raw_data {
                        LastFMAuthResponse::Success(raw_data) => {
                            let data = entity::user_connection::LastFMData {
                                token: raw_data.session.key,
                                username: raw_data.session.name,
                            };
                            Ok(serde_json::to_value(data)?)
                        }
                        LastFMAuthResponse::Error(err) => Err(eyre!(
                            "LastFM returned error code {}: {}",
                            err.code,
                            err.message
                        )),
                    }
                } else {
                    Err(eyre!("Provider {} not configured", self.to_string()))
                }
            }
        }
    }
}

pub async fn provider(
    path_provider: Path<entity::ConnectionProvider>,
    claims: Claims,
    Query(opts): Query<ProviderOptions>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Document<ConnectionResource>>, Error> {
    let provider = path_provider.inner();
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while generating connection attributes".to_owned(),
        detail: Some(e.into()),
    })?;

    Ok(Json::new(Document {
        data: DocumentData::Single(ConnectionResource {
            id: provider,
            r#type: ResourceType::Connection,
            attributes: provider
                .attributes(settings, &claims, uri, &opts)
                .await
                .map_err(|e| Error {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    title: "Error while handling connection callback".to_owned(),
                    detail: Some(e.into()),
                })?,
            meta: HashMap::new(),
            relationships: HashMap::new(),
        }),
        included: vec![],
        links: HashMap::new(),
    }))
}

#[axum_macros::debug_handler]
pub async fn callback(
    State(AppState(db)): State<AppState>,
    path_provider: Path<entity::ConnectionProvider>,
    Query(opts): Query<CallbackOptions>,
) -> Result<Response, Error> {
    let provider = path_provider.inner();
    let settings = get_settings().map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Error while handling connection callback".to_owned(),
        detail: Some(e.into()),
    })?;

    let json = provider
        .callback(settings, &opts)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Error while handling connection callback".to_owned(),
            detail: Some(e.into()),
        })?;
    let user = username(&opts.id).await.ok_or(Error {
        status: StatusCode::BAD_REQUEST,
        title: "Invalid callback id".to_owned(),
        detail: None,
    })?;
    tracing::info!(%provider, %json, %user, "User connected with provider");
    let user_connection = entity::UserConnection {
        user,
        provider,
        data: json,
    }
    .into_active_model();
    user_connection.insert(&db).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not save connection".to_owned(),
        detail: Some(e.into()),
    })?;
    if let Some(redir) = opts.redirect {
        Ok(Redirect::temporary(redir.to_string().as_str()).into_response())
    } else {
        Ok(format!(
            "Successfully logged into {}, you can now close this page",
            provider.to_string()
        )
        .into_response())
    }
}
