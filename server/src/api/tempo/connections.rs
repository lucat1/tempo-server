use axum::{
    async_trait,
    extract::{Query, State},
    http::StatusCode,
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
    documents::{
        ConnectionAttributes, ConnectionFlow, ConnectionMetaAttributes, ConnectionResource,
        Included, Meta, ResourceType,
    },
    extract::{Json, Path},
    jsonapi::{Document, DocumentData, Error},
    AppState,
};
use crate::fetch::lastfm;
use base::setting::{get_settings, Settings};
use entity::user_connection::Named;

lazy_static! {
    static ref LASTFM_URL: url::Url = url::Url::parse("https://last.fm").unwrap();
    static ref LASTFM_AUTH_URL: url::Url = url::Url::parse("http://www.last.fm/api/auth").unwrap();
    static ref LASTFM_PROFILE_URL: url::Url = url::Url::parse("https://last.fm/user/").unwrap();
    static ref ID_MAP: Arc<Mutex<HashMap<Uuid, String>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref PROVIDERS: [(entity::ConnectionProvider, ConnectionAttributes); 1] = [(
        entity::ConnectionProvider::LastFM,
        ConnectionAttributes {
            homepage: LASTFM_URL.to_owned(),
            flow: ConnectionFlow::Redirect,
        },
    )];
}

#[derive(Deserialize)]
pub struct CallbackOptions {
    pub token: String,
    pub id: Uuid,
    pub redirect: Option<Url>,
}

#[async_trait]
pub trait ProviderImpl {
    async fn url(&self, settings: &Settings, username: &str, redirect: Option<Url>) -> Result<Url>;
    async fn callback(
        &self,
        settings: &Settings,
        opts: &CallbackOptions,
    ) -> Result<serde_json::Value>;
    fn meta(&self, json: &serde_json::Value) -> Result<Meta>;
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
    ID_MAP.lock().await.remove(id)
}

#[async_trait]
impl ProviderImpl for entity::ConnectionProvider {
    async fn url(&self, settings: &Settings, username: &str, redirect: Option<Url>) -> Result<Url> {
        match self {
            entity::ConnectionProvider::LastFM => {
                if let Some(lastfm) = &settings.connections.lastfm {
                    let mut url = LASTFM_AUTH_URL.clone();
                    let mut cb_url = settings.url.clone();
                    cb_url.set_path(format!("tempo/connections/{}/callback", self.name()).as_str());
                    let id = id(username).await.to_string();
                    {
                        let cb_url_params = &mut cb_url.query_pairs_mut();
                        cb_url_params.append_pair("id", id.as_str());
                        if let Some(redir) = &redirect {
                            cb_url_params.append_pair("redirect", redir.to_string().as_str());
                        }
                    }
                    url.query_pairs_mut()
                        .append_pair("api_key", lastfm.apikey.as_str())
                        .append_pair("cb", cb_url.to_string().as_str());
                    Ok(url)
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
                let lastfm = settings
                    .connections
                    .lastfm
                    .as_ref()
                    .ok_or(eyre!("Provider {} not configured", self.name()))?;
                let mut url = lastfm::LASTFM_BASE_URL.clone();
                url.query_pairs_mut()
                    .append_pair("method", "auth.getSession")
                    .append_pair("format", "json")
                    .append_pair("api_key", lastfm.apikey.as_str())
                    .append_pair("token", opts.token.as_str());
                let signature = lastfm::signature(url.query_pairs(), lastfm.shared_secret.as_str());
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
            }
        }
    }
    fn meta(&self, json: &serde_json::Value) -> Result<Meta> {
        match self {
            entity::ConnectionProvider::LastFM => {
                let data: entity::user_connection::LastFMData =
                    serde_json::from_value(json.to_owned())?;
                Ok(Meta::Connection(ConnectionMetaAttributes {
                    profile_url: LASTFM_PROFILE_URL.join(data.username.as_str())?,
                    username: data.username,
                }))
            }
        }
    }
}

pub async fn connections() -> Result<Json<Document<ConnectionResource, Included>>, Error> {
    Ok(Json(Document {
        data: DocumentData::Multi(
            PROVIDERS
                .iter()
                .map(|(id, attrs)| ConnectionResource {
                    id: id.to_owned(),
                    r#type: ResourceType::Connection,
                    attributes: attrs.to_owned(),
                    meta: None,
                    relationships: HashMap::new(),
                })
                .collect(),
        ),
        included: vec![],
        links: HashMap::new(),
    }))
}

pub async fn connection(
    Path(provider): Path<entity::ConnectionProvider>,
) -> Result<Json<Document<ConnectionResource, Included>>, Error> {
    let (id, attrs) = PROVIDERS
        .iter()
        .find(|(id, _)| id == &provider)
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Unkown connection provider".to_owned(),
            detail: None,
        })?;

    Ok(Json(Document {
        data: DocumentData::Single(ConnectionResource {
            id: id.to_owned(),
            r#type: ResourceType::Connection,
            attributes: attrs.to_owned(),
            meta: None,
            relationships: HashMap::new(),
        }),
        included: vec![],
        links: HashMap::new(),
    }))
}

pub async fn callback(
    State(AppState(db)): State<AppState>,
    Path(provider): Path<entity::ConnectionProvider>,
    Query(opts): Query<CallbackOptions>,
) -> Result<Response, Error> {
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
            provider.name()
        )
        .into_response())
    }
}
