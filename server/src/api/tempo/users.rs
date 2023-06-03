use std::collections::HashMap;

use sea_orm::{ConnectionTrait, DbErr, LoaderTrait};

use crate::api::{
    documents::{UserAttributes, UserRelation},
    jsonapi::{
        Included, Related, Relation, Relationship, ResourceIdentifier, ResourceType, UserResource,
    },
};

#[derive(Default)]
pub struct UserRelated {
    scrobbles: Vec<entity::Scrobble>,
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
    Ok(entities
        .load_many(entity::ScrobbleEntity, db)
        .await?
        .into_iter()
        .map(|scrobbles| UserRelated { scrobbles })
        .collect::<Vec<_>>())
}

pub fn entity_to_resource(entity: &entity::User, related: &UserRelated) -> UserResource {
    let UserRelated { scrobbles } = related;
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

pub fn entity_to_included(entity: &entity::User, related: &UserRelated) -> Included {
    Included::User(entity_to_resource(entity, related))
}
