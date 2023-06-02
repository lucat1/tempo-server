use std::collections::HashMap;

use axum::extract::{OriginalUri, Path, State};
use axum::http::{Request, StatusCode};
use axum::{body::Body, response::IntoResponse};
use sea_orm::{
    ColumnTrait, ConnectionTrait, CursorTrait, DbErr, EntityTrait, LoaderTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::{artists, mediums};
use crate::api::jsonapi::ScrobbleResource;
use crate::api::{
    auth::Claims,
    documents::{
        ArtistCreditAttributes, RecordingAttributes, TrackAttributes, TrackInclude, TrackRelation,
    },
    extract::Json,
    jsonapi::{
        dedup, links_from_resource, make_cursor, Document, DocumentData, Error, Included, Meta,
        Query, Related, Relation, Relationship, ResourceIdentifier, ResourceType, TrackResource,
    },
    AppState,
};

#[axum_macros::debug_handler]
pub async fn insert_scrobbles(
    State(AppState(db)): State<AppState>,
    claims: Claims,
    json_scrobbles: Json<ScrobbleResource>,
) -> Result<Json<Document<ScrobbleResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    // let scrobbles = match json_scrobbles.inner() {
    //     DocumentData::Multi(v) => v,
    //     DocumentData::Single(r) => vec![r],
    // };
    let scrobbles = vec![json_scrobbles.inner()];
    tracing::info!(user = %claims.username, ?scrobbles, "Scrobbling");
    Err(Error {
        status: StatusCode::NOT_IMPLEMENTED,
        title: "Not implemented yet".to_string(),
        detail: None,
    })
}

pub async fn scrobbles(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::TrackColumn, TrackInclude, uuid::Uuid>,
    OriginalUri(uri): OriginalUri,
    claims: Claims,
) -> Result<Json<Vec<entity::Scrobble>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let mut _scrobbles_cursor = entity::ScrobbleEntity::find()
        .filter(ColumnTrait::eq(
            &entity::ScrobbleColumn::User,
            claims.username,
        ))
        .cursor_by(entity::TrackColumn::Id);
    let scrobbles_cursor = make_cursor(&mut _scrobbles_cursor, &opts.page);
    let scrobbles = scrobbles_cursor.all(&tx).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all tracks".to_string(),
        detail: Some(e.into()),
    })?;
    Ok(Json::new(scrobbles))
}
