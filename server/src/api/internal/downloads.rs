use eyre::Result;
use fs_extra::dir::get_size;
use std::{collections::HashMap, fs::read_dir, path::PathBuf};

use crate::api::{
    extract::{Json, Path},
    internal::documents::{
        DirectoryAttributes, DirectoryRelation, DirectoryResource, FileEntry, InternalResourceType,
        ResourceType,
    },
    jsonapi::{Document, DocumentData, Related, Relation, Relationship, ResourceIdentifier},
    Error,
};
use base::setting::{get_settings, Settings};

enum Entry {
    Directory(DirectoryResource),
    File(FileEntry),
}

trait IsFile {
    fn is_file(&self) -> bool;
}

impl IsFile for Entry {
    fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
}

pub fn abs_path(settings: &Settings, path: Option<PathBuf>) -> Result<PathBuf, Error> {
    let downloads = settings.downloads.clone();
    let abs_path = match path {
        None => Ok(downloads.to_owned()),
        Some(path) => {
            if path.is_relative() {
                Ok(downloads.join(path))
            } else {
                Err(Error::BadRequest(Some("Path is not relative".to_string())))
            }
        }
    }?;
    Ok(abs_path)
}

pub async fn list(
    path_param: Option<Path<PathBuf>>,
) -> Result<Json<Document<DirectoryResource, DirectoryResource>>, Error> {
    let path = path_param.map(|Path(p)| p);
    let settings = get_settings()?;
    let abs_path = abs_path(settings, path)?;
    tracing::info!(path = ?abs_path, "Probing download directory");

    let raw_files = read_dir(&abs_path).map_err(|_| Error::NotFound(None))?;
    let (files, directories): (Vec<Entry>, Vec<Entry>) = raw_files
        .filter_map(|f| f.ok())
        .filter_map(|f| -> Option<Entry> {
            if f.metadata().ok()?.is_file() {
                Some(Entry::File(FileEntry {
                    name: f.file_name().to_string_lossy().to_string(),
                    path: f
                        .path()
                        .strip_prefix(&settings.downloads)
                        .ok()?
                        .to_path_buf(),
                    size: get_size(f.path()).ok()?,
                }))
            } else {
                let rel = f
                    .path()
                    .strip_prefix(&settings.downloads)
                    .ok()?
                    .to_path_buf();
                Some(Entry::Directory(DirectoryResource {
                    id: urlencoding::encode(rel.to_string_lossy().to_string().as_str()).to_string(),
                    r#type: ResourceType::Internal(InternalResourceType::Directory),
                    attributes: DirectoryAttributes {
                        name: f
                            .path()
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or("/".to_string()),
                        path: rel,
                        files: Vec::new(),
                    },
                    relationships: HashMap::new(),
                    meta: None,
                }))
            }
        })
        .partition(IsFile::is_file);
    let rel = abs_path
        .strip_prefix(&settings.downloads)
        .map_err(|_| Error::Internal(None))?
        .to_path_buf();
    let files = files
        .into_iter()
        .filter_map(|e| match e {
            Entry::File(f) => Some(f),
            _ => None,
        })
        .collect();
    let directory_relationship = Relationship {
        data: Relation::Multi(
            directories
                .iter()
                .filter_map(|e| match e {
                    Entry::Directory(f) => Some(Related::String(ResourceIdentifier {
                        r#type: ResourceType::Internal(InternalResourceType::Directory),
                        id: f.id.to_owned(),
                        meta: None,
                    })),
                    _ => None,
                })
                .collect(),
        ),
    };
    let included = directories
        .into_iter()
        .filter_map(|e| match e {
            Entry::Directory(f) => Some(f),
            _ => None,
        })
        .collect();

    Ok(Json(Document {
        data: DocumentData::Single(DirectoryResource {
            id: urlencoding::encode(rel.to_string_lossy().to_string().as_str()).to_string(),
            r#type: ResourceType::Internal(InternalResourceType::Directory),
            attributes: DirectoryAttributes {
                name: abs_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("/".to_string()),
                path: rel,
                files,
            },
            relationships: [(DirectoryRelation::Directories, directory_relationship)].into(),
            meta: None,
        }),
        included,
        links: HashMap::new(),
    }))
}
