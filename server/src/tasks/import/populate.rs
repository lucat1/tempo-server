use eyre::{bail, eyre, Result, WrapErr};
use image::{
    imageops::{resize, FilterType},
    io::Reader as ImageReader,
    DynamicImage, ImageOutputFormat,
};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{io::Cursor, path::PathBuf, str::FromStr, sync::Arc};
use strfmt::strfmt;
use taskie_client::{InsertTask, Task as TaskieTask, TaskKey};
use time::Duration;
use uuid::Uuid;

use crate::{
    fetch::{
        deezer, itunes,
        musicbrainz::{self, MB_BASE_URL},
    },
    import::{CombinedSearchResults, UNKNOWN_ARTIST},
    tasks::{push, TaskName},
};
use base::{
    setting::{get_settings, ArtProvider, Settings},
    util::{mkdirp, path_to_str},
};
use entity::{
    conflict::{
        ARTIST_CONFLICT, ARTIST_CREDIT_CONFLICT, ARTIST_CREDIT_RELEASE_CONFLICT,
        ARTIST_CREDIT_TRACK_CONFLICT, ARTIST_TRACK_RELATION_CONFLICT, IMAGE_CONFLICT_1,
        IMAGE_CONFLICT_2, IMAGE_RELEASE_CONFLICT, MEDIUM_CONFLICT, RELEASE_CONFLICT,
        TRACK_CONFLICT,
    },
    full::{ArtistInfo, GetArtistCredits},
    IgnoreNone,
};
use tag::{sanitize_map, tag_to_string_map, tags_from_full_release, PictureType};

pub async fn get_cover(
    settings: &Settings,
    cover: &entity::import::Cover,
) -> Result<(Vec<u8>, (u32, u32))> {
    let req = Request::new(Method::GET, cover.url.parse()?);
    let res = match cover.provider {
        ArtProvider::Itunes => itunes::send_request(req).await,
        ArtProvider::Deezer => deezer::send_request(req).await,
        ArtProvider::CoverArtArchive => musicbrainz::send_request(req).await,
    }?;
    if !res.status().is_success() {
        bail!(
            "Fetch request for cover art returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let bytes = res.bytes().await?;
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;
    let resized =
        if settings.library.art.width < img.width() || settings.library.art.height < img.height() {
            let converted = resize(
                &img,
                settings.library.art.width,
                settings.library.art.height,
                FilterType::Gaussian,
            );
            tracing::trace! {
                src_width = img.width(),
                src_height = img.height(),
                dst_width = converted.width(),
                dst_height = converted.height(),
                "Done scaling/converting image",
            };
            DynamicImage::ImageRgba8(converted)
        } else {
            img
        };
    let mut bytes: Vec<u8> = Vec::new();
    let format: ImageOutputFormat = settings.library.art.format.into();
    resized.write_to(&mut Cursor::new(&mut bytes), format)?;
    Ok((bytes, (resized.width(), resized.height())))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let settings = get_settings()?;
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        let import_rc = Arc::new(import);
        let selected_release = import_rc
            .selected_release
            .ok_or(eyre!("Trying to process a loading import"))?;
        let full_release = entity::full::FullRelease::new(import_rc.clone(), selected_release)?;

        let release_root = settings.library.path.join(PathBuf::from_str(
            strfmt(
                settings.library.release_name.as_str(),
                &sanitize_map(tag_to_string_map(&tags_from_full_release(&full_release)?)),
            )?
            .as_str(),
        )?);

        // Save the release and its relationships
        let mut release = full_release.get_release().clone();
        let release_id = release.id;
        release.path = Some(path_to_str(&release_root)?);
        entity::ReleaseEntity::insert(release.into_active_model())
            .on_conflict(RELEASE_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        let artists: Vec<_> = full_release
            .get_artists()?
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistEntity::insert_many(artists)
            .on_conflict(ARTIST_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        let artist_credits: Vec<_> = full_release
            .get_artist_credits()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistCreditEntity::insert_many(artist_credits)
            .on_conflict(ARTIST_CREDIT_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;
        let artist_credits_release: Vec<_> = full_release
            .get_artist_credits_release()
            .into_iter()
            .map(|a| a.clone().into_active_model())
            .collect();
        entity::ArtistCreditReleaseEntity::insert_many(artist_credits_release)
            .on_conflict(ARTIST_CREDIT_RELEASE_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;

        // Save mediums
        let mediums: Vec<_> = full_release
            .get_mediums()
            .into_iter()
            .map(|m| m.clone().into_active_model())
            .collect();
        entity::MediumEntity::insert_many(mediums)
            .on_conflict(MEDIUM_CONFLICT.to_owned())
            .exec(&tx)
            .await
            .ignore_none()?;

        mkdirp(&release_root).wrap_err(eyre!(
            "Could not create folder {:?} for release",
            release_root
        ))?;

        // Save the image if available
        let cover = if let Some(cover_i) = import_rc.selected_cover {
            let cover = import_rc.covers.0.get(cover_i as usize).ok_or(eyre!(
                "Imported release has selected a non-existant cover {} of {}",
                cover_i,
                import_rc.covers.0.len()
            ))?;
            let image_dest = release_root.join(settings.library.art.image_name.clone());
            let (buf, (width, height)) = get_cover(settings, cover).await?;
            tokio::fs::write(&image_dest, &buf).await?;

            let image_dest = path_to_str(&image_dest)?;
            let image_id = sha256::digest(&image_dest);
            let image = entity::ImageActive {
                id: ActiveValue::Set(image_id.clone()),
                path: ActiveValue::Set(image_dest.clone()),
                role: ActiveValue::Set(PictureType::CoverFront.to_string()),
                description: ActiveValue::Set(Some("Front Cover".to_string())),
                format: ActiveValue::Set(settings.library.art.format),
                width: ActiveValue::Set(width as i32),
                height: ActiveValue::Set(height as i32),
                size: ActiveValue::Set(buf.len() as i32),
            };
            entity::ImageEntity::insert(image)
                .on_conflict(IMAGE_CONFLICT_1.to_owned())
                .on_conflict(IMAGE_CONFLICT_2.to_owned())
                .exec(&tx)
                .await
                .ignore_none()?;
            let image_release = entity::ImageReleaseActive {
                image_id: ActiveValue::Set(image_id),
                release_id: ActiveValue::Set(release_id),
            };
            entity::ImageReleaseEntity::insert(image_release)
                .on_conflict(IMAGE_RELEASE_CONFLICT.to_owned())
                .exec(&tx)
                .await
                .ignore_none()?;
            Some(image_dest)
        } else {
            None
        };
        push(
            &import_rc
                .release_matches
                .0
                .get(&selected_release)
                .ok_or(eyre!("Importing an unmatched release"))?
                .assignment
                .iter()
                .map(|(src, dest)| InsertTask {
                    name: TaskName::ImportTrack,
                    payload: Some(json!(super::ImportTrack {
                        import: self.0,
                        release: release_id,
                        track: *dest,
                        source: *src,
                        cover: cover.clone(),
                    })),
                    depends_on: vec![task.id.clone()],
                    duration: Duration::seconds(120),
                })
                .collect::<Vec<_>>(),
        )
        .await?;

        Ok(tx.commit().await?)
    }
}
