use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use eyre::{eyre, Result};
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, CursorTrait, DbErr, EntityTrait, IntoActiveModel,
    LoaderTrait, PaginatorTrait, QueryFilter, TransactionTrait,
};
use std::collections::HashMap;

use super::{tracks, users};
use crate::api::documents::TrackInclude;
use crate::api::{
    auth::Claims,
    documents::{ScrobbleAttributes, ScrobbleInclude, ScrobbleRelation},
    extract::Json,
    jsonapi::{
        dedup, links_from_resource, make_cursor, Document, DocumentData, Error, Included,
        InsertDocument, InsertScrobbleResource, Page, Query, Related, Relation, Relationship,
        ResourceIdentifier, ResourceType, ScrobbleResource,
    },
    AppState,
};

pub fn entity_to_resource(entity: &entity::Scrobble) -> ScrobbleResource {
    let mut relationships = HashMap::new();
    relationships.insert(
        ScrobbleRelation::User,
        Relationship {
            data: Relation::Single(Related::String(ResourceIdentifier {
                r#type: ResourceType::User,
                id: entity.user.to_owned(),
                meta: None,
            })),
        },
    );
    relationships.insert(
        ScrobbleRelation::Track,
        Relationship {
            data: Relation::Single(Related::Uuid(ResourceIdentifier {
                r#type: ResourceType::Track,
                id: entity.track,
                meta: None,
            })),
        },
    );

    ScrobbleResource {
        r#type: ResourceType::Scrobble,
        id: entity.id,
        attributes: ScrobbleAttributes { at: entity.at },
        relationships,
        meta: HashMap::new(),
    }
}

pub fn entity_to_included(entity: &entity::Scrobble) -> Included {
    Included::Scrobble(entity_to_resource(entity))
}

fn map_to_tracks_include(include: &[ScrobbleInclude]) -> Vec<TrackInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            ScrobbleInclude::TrackArtists => Some(TrackInclude::Artists),
            ScrobbleInclude::TrackMedium => Some(TrackInclude::Medium),
            ScrobbleInclude::TrackMediumRelease => Some(TrackInclude::MediumRelease),
            ScrobbleInclude::TrackMediumReleaseArtists => Some(TrackInclude::MediumReleaseArtists),
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    entities: Vec<entity::Scrobble>,
    include: &[ScrobbleInclude],
) -> Result<Vec<Included>, DbErr>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&ScrobbleInclude::User) {
        let users = entities
            .load_one(entity::UserEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();
        let users_related = users::related(db, &users, true).await?;
        for (i, user) in users.into_iter().enumerate() {
            included.push(users::entity_to_included(&user, &users_related[i]));
        }
    }
    if include.contains(&ScrobbleInclude::Track) {
        let tracks = entities
            .load_one(entity::TrackEntity, db)
            .await?
            .into_iter()
            .flatten()
            .collect();
        let tracks_related = tracks::related(db, &tracks, true).await?;
        for (i, track) in tracks.into_iter().enumerate() {
            included.push(tracks::entity_to_included(&track, &tracks_related[i]));
        }
        let tracks_include = map_to_tracks_include(include);
        included.extend(tracks::included(db, tracks_related, &tracks_include).await?);
    }
    Ok(included)
}

pub fn resource_to_active_entity(
    resource: &InsertScrobbleResource,
) -> Result<entity::ScrobbleActive> {
    let user_relation = resource
        .relationships
        .get(&ScrobbleRelation::User)
        .ok_or(eyre!("Scrobble resource is missing the user relation"))?;
    let track_relation = resource
        .relationships
        .get(&ScrobbleRelation::Track)
        .ok_or(eyre!("Scrobble resource is missing the track relation"))?;

    let user = match &user_relation.data {
        Relation::Single(Related::String(data)) => Ok(data.id.to_owned()),
        _ => Err(eyre!("Invalid user relation")),
    }?;
    let track = match &track_relation.data {
        Relation::Single(Related::Uuid(data)) => Ok(data.id),
        _ => Err(eyre!("Invalid track relation")),
    }?;

    Ok(entity::ScrobbleActive {
        id: ActiveValue::NotSet,
        at: ActiveValue::Set(resource.attributes.at),
        user: ActiveValue::Set(user),
        track: ActiveValue::Set(track),
    })
}

async fn fetch_scrobbles<C>(
    db: &C,
    username: String,
    page: &Page<i64>,
    include: &[ScrobbleInclude],
) -> Result<(Vec<ScrobbleResource>, Vec<Included>), Error>
where
    C: ConnectionTrait,
{
    let mut _scrobbles_cursor = entity::ScrobbleEntity::find()
        .filter(ColumnTrait::eq(&entity::ScrobbleColumn::User, username))
        .cursor_by(entity::TrackColumn::Id);
    let scrobbles_cursor = make_cursor(&mut _scrobbles_cursor, page);
    let scrobbles = scrobbles_cursor.all(db).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch all tracks".to_string(),
        detail: Some(e.into()),
    })?;
    let data = scrobbles.iter().map(entity_to_resource).collect::<Vec<_>>();
    let included = included(db, scrobbles, include).await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Could not fetch the included resurces".to_string(),
        detail: Some(e.into()),
    })?;
    Ok((data, included))
}

pub async fn insert_scrobbles(
    State(AppState(db)): State<AppState>,
    claims: Claims,
    json_scrobbles: Json<InsertDocument<InsertScrobbleResource>>,
) -> Result<Json<Document<ScrobbleResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let after = entity::ScrobbleEntity::find().count(&tx).await.unwrap();
    let scrobbles = match json_scrobbles.inner().data {
        DocumentData::Multi(v) => v,
        DocumentData::Single(r) => vec![r],
    };
    let entities = scrobbles
        .iter()
        .map(resource_to_active_entity)
        .collect::<Result<Vec<_>>>()
        .map_err(|e| Error {
            status: StatusCode::BAD_REQUEST,
            title: "Invalid scrobble data".to_string(),
            detail: Some(e.into()),
        })?;
    tracing::info!(user = %claims.username, scrobbles = ?entities, "Scrobbling");
    let entities = entities
        .into_iter()
        .map(|e| e.into_active_model())
        .collect::<Vec<_>>();
    let res = entity::ScrobbleEntity::insert_many(entities)
        .exec(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Error while saving scrobbles".to_string(),
            detail: Some(e.into()),
        })?;
    tx.commit().await.map_err(|e| Error {
        status: StatusCode::BAD_REQUEST,
        title: "Could not commit scrobbles".to_string(),
        detail: Some(e.into()),
    })?;

    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let page = Page {
        size: u32::MAX,
        before: Some(res.last_insert_id + 1),
        after: Some(after as i64),
    };
    let (data, included) = fetch_scrobbles(&tx, claims.username, &page, &[]).await?;
    Ok(Json::new(Document {
        links: HashMap::new(),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

pub async fn scrobbles(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::ScrobbleColumn, ScrobbleInclude, i64>,
    OriginalUri(uri): OriginalUri,
    claims: Claims,
) -> Result<Json<Document<ScrobbleResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let (data, included) = fetch_scrobbles(&tx, claims.username, &opts.page, &opts.include).await?;
    Ok(Json::new(Document {
        links: links_from_resource(&data, opts, &uri),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

pub async fn scrobble(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::ScrobbleColumn, ScrobbleInclude, i64>,
    Path(id): Path<i64>,
) -> Result<Json<Document<ScrobbleResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let scrobble = entity::ScrobbleEntity::find_by_id(id)
        .one(&tx)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch all scrobbles".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Not found".to_string(),
            detail: Some("Not found".into()),
        })?;
    let resource = entity_to_resource(&scrobble);
    let included = included(&tx, vec![scrobble], &opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        links: HashMap::new(),
        data: DocumentData::Single(resource),
        included: dedup(included),
    }))
}