use axum::extract::State;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::read_dir, path::PathBuf};

use crate::api::{
    extract::{Json, Path},
    internal::documents::{
        DirectoryAttributes, DirectoryRelation, DirectoryResource, FileEntry, InternalResourceType,
        ResourceType,
    },
    jsonapi::{Document, DocumentData, Error, Related, Relation, Relationship, ResourceIdentifier},
    AppState,
};
use crate::tasks::TaskEntities;
use base::setting::{get_settings, Settings};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(untagged)]
pub enum UpdateType {
    #[serde(rename = "all")]
    All,

    Single(entity::UpdateType),
}

pub async fn all(
    State(AppState(db)): State<AppState>,
    update_type: Path<UpdateType>,
) -> Result<(), Error> {
    Ok(())
}

pub async fn outdated(
    State(AppState(db)): State<AppState>,
    update_type: Path<UpdateType>,
) -> Result<(), Error> {
    println!(
        "{:?}",
        crate::tasks::artist_description::Data::all(&db).await
    );
    println!(
        "{:?}",
        crate::tasks::artist_description::Data::outdated(&db).await
    );
    Ok(())
}
