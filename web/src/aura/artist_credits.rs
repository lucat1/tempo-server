use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::EntityTrait;

use super::AppState;
use crate::documents::{ArtistCreditAttributes, ArtistCreditRelation};
use crate::jsonapi::{
    ArtistCreditResource, Document, DocumentData, Error, Related, Relation, RelationData,
    Relationship, ResourceType,
};

pub fn entity_to_resource(artist_credit: entity::ArtistCredit) -> ArtistCreditResource {
    ArtistCreditResource {
        id: artist_credit.id,
        r#type: ResourceType::ArtistCredit,
        attributes: ArtistCreditAttributes {
            join_phrase: artist_credit.join_phrase,
        },
        relationships: [(
            ArtistCreditRelation::Artist,
            Relationship {
                data: Relation::Single(Related::Artist(RelationData {
                    r#type: ResourceType::Artist,
                    id: artist_credit.artist_id,
                })),
            },
        )]
        .into(),
    }
}

pub async fn artist_credit(
    State(AppState(db)): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Document<ArtistCreditResource>>, Error> {
    let artist_credit = entity::ArtistCreditEntity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Could not fetch the requried artist credit".to_string(),
            detail: Some(e.into()),
        })?
        .ok_or(Error {
            status: StatusCode::NOT_FOUND,
            title: "Artist credit not found".to_string(),
            detail: None,
        })?;
    Ok(Json(Document {
        data: DocumentData::Single(entity_to_resource(artist_credit)),
        included: Vec::new(),
    }))
}
