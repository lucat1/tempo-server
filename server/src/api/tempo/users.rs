use axum::{
    extract::State,
    headers::{Header, HeaderValue, Location},
    http::StatusCode,
    TypedHeader,
};
use sea_orm::{ConnectionTrait, DbErr, EntityTrait, LoaderTrait, TransactionTrait};
use serde::Deserialize;
use std::collections::HashMap;

use crate::api::{
    auth::Claims,
    documents::{
        dedup, Included, Meta, ResourceType, ScrobbleInclude, UserAttributes, UserFilter,
        UserInclude, UserRelation, UserResource,
    },
    extract::{Json, Path},
    jsonapi::{
        Document, DocumentData, Error, InsertManyRelation, Query, Related, Relation, Relationship,
        ResourceIdentifier,
    },
    tempo::{connections::ProviderImpl, scrobbles},
    AppState,
};
use base::setting::get_settings;

#[derive(Default)]
pub struct UserRelated {
    scrobbles: Vec<entity::Scrobble>,
    connections: Vec<entity::UserConnection>,
}

pub async fn related<C>(
    db: &C,
    entities: &[entity::User],
    _light: bool,
) -> Result<Vec<UserRelated>, DbErr>
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
            _ => None,
        })
        .collect()
}

pub async fn included<C>(
    db: &C,
    related: Vec<UserRelated>,
    include: &[UserInclude],
) -> Result<Vec<Included>, DbErr>
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
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch user".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Not found".to_string(),
            detail: Some("Not found".into()),
        })?;
    let related = related(db, &[user.to_owned()], false)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch entites related to the user".to_string(),
            detail: Some(e.into()),
        })?
        .into_iter()
        .next()
        .unwrap_or_default();
    let data = entity_to_resource(&user, &related);
    let included = included(db, vec![related], include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok((data, included))
}

pub async fn user(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<UserFilter, entity::UserColumn, UserInclude, String>,
    username_path: Path<String>,
) -> Result<Json<Document<UserResource, Included>>, Error> {
    let username = username_path.inner();
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let (data, included) = fetch_user(&tx, username, &opts.include).await?;
    Ok(Json::new(Document {
        links: HashMap::new(),
        data: DocumentData::Single(data),
        included: dedup(included),
    }))
}

pub async fn relation(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<UserFilter, entity::UserColumn, UserInclude, String>,
    user_rel_path: Path<(String, UserRelation)>,
) -> Result<Json<Document<Related<ResourceType, Meta>, Included>>, Error> {
    let (username, relation) = user_rel_path.inner();
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let (data, included) = fetch_user(&tx, username, &opts.include).await?;
    let related = data
        .relationships
        .get(&relation)
        .map(|r| r.data.to_owned())
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "No relationship data".to_string(),
            detail: None,
        })?;
    Ok(Json::new(Document {
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
    user_rel_path: Path<(String, UserRelation)>,
    body: Json<
        InsertExactlyOneRelation<
            ResourceIdentifier<ResourceType, entity::ConnectionProvider, Meta>,
        >,
    >,
) -> Result<(StatusCode, TypedHeader<Location>), Error> {
    let (username, relation) = user_rel_path.inner();
    if claims.username != username {
        return Err(Error {
            status: StatusCode::UNAUTHORIZED,
            title: "You can only edit your own connections".to_string(),
            detail: None,
        });
    }
    if relation != UserRelation::Connections {
        return Err(Error {
            status: StatusCode::BAD_REQUEST,
            title: "Users can only edit connection relationships".to_string(),
            detail: None,
        });
    }

    let relation = body.inner();

    let connection =
        entity::UserConnectionEntity::find_by_id((claims.username, relation.data[0].id))
            .one(&db)
            .await
            .map_err(|e| Error {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                title: "Could not check if the connectino already exists".to_string(),
                detail: Some(e.into()),
            })?;
    if connection.is_some() {
        Err(Error {
            status: StatusCode::NOT_MODIFIED,
            title: "Connection already enstablished".to_string(),
            detail: None,
        })
    } else {
        let settings = get_settings().map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Error while handling connection callback".to_owned(),
            detail: Some(e.into()),
        })?;

        // TODO: redirect url
        let url = relation.data[0]
            .id
            .url(settings, username.as_str(), None)
            .await
            .map_err(|e| Error {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                title: "Could not generate connection URL".to_string(),
                detail: Some(e.into()),
            })?;
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
    user_rel_path: Path<(String, UserRelation)>,
    body: Json<
        InsertManyRelation<ResourceIdentifier<ResourceType, entity::ConnectionProvider, Meta>>,
    >,
) -> Result<StatusCode, Error> {
    let (username, relation) = user_rel_path.inner();
    if claims.username != username {
        return Err(Error {
            status: StatusCode::UNAUTHORIZED,
            title: "You can only edit your own connections".to_string(),
            detail: None,
        });
    }
    // TODO: support deleting scrobbles and others
    if relation != UserRelation::Connections {
        return Err(Error {
            status: StatusCode::BAD_REQUEST,
            title: "Users can only edit connection relationships".to_string(),
            detail: None,
        });
    }

    let relation = body.inner();
    entity::UserConnectionEntity::delete_by_id((claims.username, relation.data[0].id))
        .exec(&db)
        .await
        .map_err(|e| Error {
            status: StatusCode::FORBIDDEN,
            title: "Could not delete the requested relations".to_string(),
            detail: Some(e.into()),
        })?;

    Ok(StatusCode::OK)
}
