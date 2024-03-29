use axum::extract::{OriginalUri, State};
use eyre::{eyre, Result};
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, CursorTrait, EntityTrait, LoaderTrait,
    PaginatorTrait, QueryFilter, TransactionTrait, Value,
};
use serde_json::json;
use std::collections::HashMap;
use taskie_client::InsertTask;
use time::Duration;
use uuid::Uuid;

use crate::api::{
    documents::{
        Included, InsertScrobbleResource, ResourceType, ScrobbleAttributes, ScrobbleFilter,
        ScrobbleInclude, ScrobbleRelation, ScrobbleResource, TrackInclude,
    },
    extract::{Claims, Json, Path},
    jsonapi::{
        links_from_resource, make_cursor, Document, DocumentData, InsertDocument, Page, Query,
        Related, Relation, Relationship, ResourceIdentifier,
    },
    tempo::{tracks, users},
    AppState, Error,
};
use crate::tasks::{self, TaskName};
use base::util::dedup;

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
        meta: None,
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
            ScrobbleInclude::TrackGenres => Some(TrackInclude::Genres),
            ScrobbleInclude::TrackMedium => Some(TrackInclude::Medium),
            ScrobbleInclude::TrackMediumRelease => Some(TrackInclude::MediumRelease),
            ScrobbleInclude::TrackMediumReleaseArtists => Some(TrackInclude::MediumReleaseArtists),
            ScrobbleInclude::TrackMediumReleaseGenres => Some(TrackInclude::MediumReleaseGenres),
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    entities: Vec<entity::Scrobble>,
    include: &[ScrobbleInclude],
) -> Result<Vec<Included>, Error>
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
            .collect::<Vec<_>>();
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
            .collect::<Vec<_>>();
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
    let scrobbles = scrobbles_cursor.all(db).await?;
    let data = scrobbles.iter().map(entity_to_resource).collect::<Vec<_>>();
    let included = included(db, scrobbles, include).await?;
    Ok((data, included))
}

pub async fn schedule_scrobble_tasks<I>(username: &str, tracks: I) -> Result<()>
where
    I: Iterator<Item = (Uuid, time::OffsetDateTime)>,
{
    for (track_id, time) in tracks.into_iter() {
        // TODO: use user's setting to determine the subset of connections he wants
        // to scrobble to.
        let data = tasks::scrobble::Data {
            provider: entity::ConnectionProvider::LastFM,
            username: username.to_owned(),
            time,
            track_id,
        };
        tasks::push(&[InsertTask {
            name: TaskName::Scrobble,
            payload: Some(json!(data)),
            depends_on: Vec::new(),
            duration: Duration::seconds(60),
        }])
        .await?;
    }
    Ok(())
}

pub async fn insert_scrobbles(
    State(AppState(db)): State<AppState>,
    claims: Claims,
    Json(scrobbles): Json<InsertDocument<InsertScrobbleResource>>,
) -> Result<Json<Document<ScrobbleResource, Included>>, Error> {
    let tx = db.begin().await?;
    let after = entity::ScrobbleEntity::find().count(&tx).await?;
    let scrobbles = match scrobbles.data {
        DocumentData::Multi(v) => v,
        DocumentData::Single(r) => vec![r],
    };
    for scrobble in scrobbles.iter() {
        if let Some(Relationship {
            data: Relation::Single(Related::String(data)),
        }) = scrobble.relationships.get(&ScrobbleRelation::User)
        {
            if data.id != claims.username {
                return Err(Error::BadRequest(Some(
                    "You cannot insert a scrobble for another user".to_string(),
                )));
            }
        }
    }
    let entities = scrobbles
        .iter()
        .map(resource_to_active_entity)
        .collect::<Result<Vec<_>>>()
        // TODO: once we get an error for resource to entity conversion, handle that properly
        .map_err(|_| Error::BadRequest(Some("Invalid scrobble data".to_string())))?;
    tracing::info!(user = %claims.username, scrobbles = ?entities, "Scrobbling");
    let res = entity::ScrobbleEntity::insert_many(entities.clone())
        .exec(&tx)
        .await?;
    tx.commit().await?;

    let tx = db.begin().await?;

    schedule_scrobble_tasks(
        claims.username.as_str(),
        entities
            .iter()
            .map(|e| (e.track.clone().into_value(), e.at.clone().into_value()))
            .filter_map(|i| match i {
                (
                    Some(Value::Uuid(Some(id))),
                    Some(Value::TimeDateTimeWithTimeZone(Some(time))),
                ) => Some((*id, *time)),
                _ => None,
            }),
    )
    .await
    // TODO: once we convert the error here, handle it properly
    .map_err(|_| Error::BadRequest(None))?;

    let page = Page {
        size: u32::MAX,
        before: Some(res.last_insert_id + 1),
        after: Some(after as i64),
    };
    let (data, included) = fetch_scrobbles(&tx, claims.username, &page, &[]).await?;
    Ok(Json(Document {
        links: HashMap::new(),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

pub async fn scrobbles(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ScrobbleFilter, entity::ScrobbleColumn, ScrobbleInclude, i64>,
    OriginalUri(uri): OriginalUri,
    claims: Claims,
) -> Result<Json<Document<ScrobbleResource, Included>>, Error> {
    let tx = db.begin().await?;
    let (data, included) = fetch_scrobbles(&tx, claims.username, &opts.page, &opts.include).await?;
    Ok(Json(Document {
        links: links_from_resource(&data, opts, &uri),
        data: DocumentData::Multi(data),
        included: dedup(included),
    }))
}

pub async fn scrobble(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<ScrobbleFilter, entity::ScrobbleColumn, ScrobbleInclude, i64>,
    Path(id): Path<i64>,
) -> Result<Json<Document<ScrobbleResource, Included>>, Error> {
    let tx = db.begin().await?;
    let scrobble = entity::ScrobbleEntity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or(Error::NotFound(None))?;
    let resource = entity_to_resource(&scrobble);
    let included = included(&tx, vec![scrobble], &opts.include).await?;
    Ok(Json(Document {
        links: HashMap::new(),
        data: DocumentData::Single(resource),
        included: dedup(included),
    }))
}
