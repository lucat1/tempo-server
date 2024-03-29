use axum::{
    extract::State,
    headers::{Header, HeaderValue, Location},
    http::StatusCode,
    TypedHeader,
};
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, TransactionTrait};
use serde::Deserialize;
use std::collections::HashMap;

use crate::api::{
    documents::{
        Included, Meta, ResourceType, ScrobbleInclude, UserAttributes, UserFilter, UserInclude,
        UserRelation, UserResource,
    },
    extract::{Claims, Json, Path},
    jsonapi::{
        Document, DocumentData, InsertManyRelation, Query, Related, Relation, Relationship,
        ResourceIdentifier,
    },
    tempo::{connections::ProviderImpl, scrobbles},
    AppState, Error,
};
use base::setting::get_settings;
use base::util::dedup;

#[derive(Default)]
pub struct UserRelated {
    scrobbles: Vec<entity::Scrobble>,
    connections: Vec<entity::UserConnection>,
}

pub async fn related<C>(
    db: &C,
    entities: &[entity::User],
    _light: bool,
) -> Result<Vec<UserRelated>, Error>
where
    C: ConnectionTrait,
{
    // TODO: limit number of returned scrobbles. Limit even more when light = true
    let scrobbles = entities.load_many(entity::ScrobbleEntity, db).await?;
    let connections = entities.load_many(entity::UserConnectionEntity, db).await?;
    let mut result = Vec::with_capacity(entities.len());
    for i in 0..entities.len() {
        let scrobbles = &scrobbles[i];
        let connections = &connections[i];
        result.push(UserRelated {
            scrobbles: scrobbles.to_vec(),
            connections: connections.to_vec(),
        })
    }
    Ok(result)
}

pub fn entity_to_resource(entity: &entity::User, related: &UserRelated) -> UserResource {
    let UserRelated {
        scrobbles,
        connections,
    } = related;
    let mut relationships = HashMap::new();
    if !scrobbles.is_empty() {
        relationships.insert(
            UserRelation::Scrobbles,
            Relationship {
                data: Relation::Multi(
                    scrobbles
                        .iter()
                        .map(|s| {
                            Related::Int(ResourceIdentifier {
                                r#type: ResourceType::Scrobble,
                                id: s.id,
                                meta: None,
                            })
                        })
                        .collect(),
                ),
            },
        );
    }
    if !connections.is_empty() {
        relationships.insert(
            UserRelation::Connections,
            Relationship {
                data: Relation::Multi(
                    connections
                        .iter()
                        .map(|s| {
                            Related::ConnectionProvider(ResourceIdentifier {
                                r#type: ResourceType::Connection,
                                id: s.provider,
                                meta: s.provider.meta(&s.data).ok(),
                            })
                        })
                        .collect(),
                ),
            },
        );
    }

    UserResource {
        r#type: ResourceType::User,
        id: entity.username.to_owned(),
        attributes: UserAttributes {
            first_name: entity.first_name.to_owned(),
            last_name: entity.last_name.to_owned(),
        },
        meta: None,
        relationships,
    }
}

fn map_to_scrobbles_include(include: &[UserInclude]) -> Vec<ScrobbleInclude> {
    include
        .iter()
        .filter_map(|i| match *i {
            UserInclude::ScrobblesTracks => Some(ScrobbleInclude::Track),
            UserInclude::ScrobblesTracksArtists => Some(ScrobbleInclude::TrackArtists),
            UserInclude::ScrobblesTracksMedium => Some(ScrobbleInclude::TrackMedium),
            UserInclude::ScrobblesTracksMediumRelease => Some(ScrobbleInclude::TrackMediumRelease),
            UserInclude::ScrobblesTracksMediumReleaseArtists => {
                Some(ScrobbleInclude::TrackMediumReleaseArtists)
            }
            UserInclude::ScrobblesTracksMediumReleaseGenres => {
                Some(ScrobbleInclude::TrackMediumReleaseGenres)
            }
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    related: Vec<UserRelated>,
    include: &[UserInclude],
) -> Result<Vec<Included>, Error>
where
    C: ConnectionTrait,
{
    let mut included = Vec::new();
    if include.contains(&UserInclude::Scrobbles) {
        let scrobbles = related
            .into_iter()
            .flat_map(|r| r.scrobbles)
            .collect::<Vec<_>>();
        let scrobbles_include = map_to_scrobbles_include(include);
        included.extend(scrobbles::included(db, scrobbles.to_owned(), &scrobbles_include).await?);

        included.extend(
            scrobbles
                .iter()
                .map(scrobbles::entity_to_included)
                .collect::<Vec<Included>>(),
        );
    }
    Ok(included)
}

pub fn entity_to_included(entity: &entity::User, related: &UserRelated) -> Included {
    Included::User(entity_to_resource(entity, related))
}

async fn fetch_user<C>(
    db: &C,
    username: String,
    include: &[UserInclude],
) -> Result<(UserResource, Vec<Included>), Error>
where
    C: ConnectionTrait,
{
    let user = entity::UserEntity::find_by_id(username)
        .one(db)
        .await?
        .ok_or(Error::NotFound(None))?;
    let related = related(db, &[user.to_owned()], false)
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();
    let data = entity_to_resource(&user, &related);
    let included = included(db, vec![related], include).await?;
    Ok((data, included))
}

pub async fn user(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<UserFilter, entity::UserColumn, UserInclude, String>,
    Path(username): Path<String>,
) -> Result<Json<Document<UserResource, Included>>, Error> {
    let tx = db.begin().await?;
    let (data, included) = fetch_user(&tx, username, &opts.include).await?;
    Ok(Json(Document {
        links: HashMap::new(),
        data: DocumentData::Single(data),
        included: dedup(included),
    }))
}

pub async fn relation(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<UserFilter, entity::UserColumn, UserInclude, String>,
    Path((username, relation)): Path<(String, UserRelation)>,
) -> Result<Json<Document<Related<ResourceType, Meta>, Included>>, Error> {
    let tx = db.begin().await?;
    let (data, included) = fetch_user(&tx, username, &opts.include).await?;
    let related = data
        .relationships
        .get(&relation)
        .map(|r| r.data.to_owned())
        .ok_or(Error::NotFound(None))?;
    Ok(Json(Document {
        links: HashMap::new(),
        data: match related {
            Relation::Multi(r) => DocumentData::Multi(r),
            Relation::Single(r) => DocumentData::Single(r),
        },
        included: dedup(included),
    }))
}

#[derive(Deserialize)]
pub struct InsertExactlyOneRelation<R> {
    pub data: [R; 1],
}

pub async fn post_relation(
    State(AppState(db)): State<AppState>,
    claims: Claims,
    Path((username, relation_kind)): Path<(String, UserRelation)>,
    Json(relation): Json<
        InsertExactlyOneRelation<
            ResourceIdentifier<ResourceType, entity::ConnectionProvider, Meta>,
        >,
    >,
) -> Result<(StatusCode, TypedHeader<Location>), Error> {
    if claims.username != username {
        return Err(Error::Unauthorized(None));
    }
    if relation_kind != UserRelation::Connections {
        return Err(Error::BadRequest(Some(
            "Users can only edit connection relationships".to_string(),
        )));
    }

    let connection =
        entity::UserConnectionEntity::find_by_id((claims.username, relation.data[0].id))
            .one(&db)
            .await?;
    if connection.is_some() {
        Err(Error::NotModified)
    } else {
        let settings = get_settings()?;

        // TODO: redirect url
        let url = relation.data[0]
            .id
            .url(settings, username.as_str(), None)
            .await?;
        Ok((
            StatusCode::CREATED,
            TypedHeader(
                Location::decode(
                    &mut [HeaderValue::from_str(url.to_string().as_str()).unwrap()].iter(),
                )
                .unwrap(),
            ),
        ))
    }
}

pub async fn delete_relation(
    State(AppState(db)): State<AppState>,
    claims: Claims,
    Path((username, relation_kind)): Path<(String, UserRelation)>,
    Json(relation): Json<
        InsertManyRelation<ResourceIdentifier<ResourceType, entity::ConnectionProvider, Meta>>,
    >,
) -> Result<StatusCode, Error> {
    if claims.username != username {
        return Err(Error::Unauthorized(None));
    }
    // TODO: support deleting scrobbles and others
    if relation_kind != UserRelation::Connections {
        return Err(Error::BadRequest(Some(
            "Users can only edit connection relationships".to_string(),
        )));
    }

    entity::UserConnectionEntity::delete_by_id((claims.username, relation.data[0].id))
        .exec(&db)
        .await?;

    Ok(StatusCode::OK)
}
