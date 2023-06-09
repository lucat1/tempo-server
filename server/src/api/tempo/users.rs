use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::{ConnectionTrait, DbErr, EntityTrait, LoaderTrait, TransactionTrait};
use std::collections::HashMap;

use crate::api::{
    documents::{ScrobbleInclude, UserAttributes, UserInclude, UserRelation},
    extract::Json,
    jsonapi::{
        dedup, Document, DocumentData, Error, Included, Query, Related, Relation, Relationship,
        ResourceIdentifier, ResourceType, UserResource,
    },
    tempo::{connections, scrobbles},
    AppState,
};

#[derive(Default)]
pub struct UserRelated {
    scrobbles: Vec<entity::Scrobble>,
    connections: Vec<entity::UserConnection>,
}

pub async fn related<C>(
    db: &C,
    entities: &Vec<entity::User>,
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
                                meta: None,
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
        meta: HashMap::new(),
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
    // if include.contains(&UserInclude::Connections) {
    //     included.extend(
    //         related
    //             .iter()
    //             .flat_map(|r| r.connections)
    //             .map(|s| connections::entity_to_included(&s))
    //             .collect::<Vec<_>>(),
    //     );
    // }
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

pub async fn user(
    State(AppState(db)): State<AppState>,
    Query(opts): Query<entity::UserColumn, UserInclude, String>,
    Path(username): Path<String>,
) -> Result<Json<Document<UserResource>>, Error> {
    let tx = db.begin().await.map_err(|e| Error {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        title: "Couldn't begin database transaction".to_string(),
        detail: Some(e.into()),
    })?;
    let user = entity::UserEntity::find_by_id(username)
        .one(&tx)
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
    let related = related(&tx, &vec![user.to_owned()], false)
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
    let included = included(&tx, vec![related], &opts.include)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the included resurces".to_string(),
            detail: Some(e.into()),
        })?;
    Ok(Json::new(Document {
        links: HashMap::new(),
        data: DocumentData::Single(data),
        included: dedup(included),
    }))
}
