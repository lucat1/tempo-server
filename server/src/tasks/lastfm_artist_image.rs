use eyre::{bail, eyre, Result, WrapErr};
use image::io::Reader as ImageReader;
use lazy_static::lazy_static;
use reqwest::{get, Method, Request};
use scraper::{Html, Selector};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, JoinType, QueryFilter, QuerySelect,
    RelationTrait, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{io::Cursor, path::PathBuf};
use strfmt::strfmt;
use tag::PictureType;
use taskie_client::{Task as TaskieTask, TaskKey};
use uuid::Uuid;

use crate::fetch::lastfm::send_request;
use crate::tasks::TaskName;
use base::setting::get_settings;
use base::util::{mkdirp, path_to_str};
use base::ImageFormat;
use entity::IgnoreNone;
use tag::{map::tags_from_artist, tag_to_string_map};

lazy_static! {
    static ref IMAGE_SELECTOR: Selector =
        Selector::parse("ul.image-list li.image-list-item-wrapper a.image-list-item").unwrap();
    static ref IMAGE_DESCRIPTION_SELECTOR: Selector =
        Selector::parse(".gallery-image-description").unwrap();
}
static LASTFM_IMAGE_ATTEMPT_SIZE: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data(Uuid, String);

async fn get_html(url: url::Url) -> Result<String> {
    let res = send_request(Request::new(Method::GET, url)).await?;
    if !res.status().is_success() {
        bail!(
            "Last.fm request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    res.text()
        .await
        .wrap_err(eyre!("Could not read response as text"))
}

struct UrlData {
    image_page: url::Url,
    image_url: url::Url,
    image_id: String,
}

fn img_url(url: &url::Url) -> Result<UrlData> {
    let src_url = url.clone();
    if let Some(path) = url.path_segments() {
        if let Some(id) = path.last() {
            let url = format!(
                "https://lastfm.freetls.fastly.net/i/u/{}x0/{}.jpg#{}",
                LASTFM_IMAGE_ATTEMPT_SIZE, id, id
            )
            .parse()?;
            return Ok(UrlData {
                image_page: src_url,
                image_url: url,
                image_id: id.to_string(),
            });
        }
    }

    Err(eyre!(
        "Could not extract image id from last.fm url: {}",
        src_url
    ))
}

fn extract_description(html: &str) -> Option<String> {
    let html = Html::parse_document(html);
    html.select(&IMAGE_DESCRIPTION_SELECTOR)
        .next()
        .map(|p| p.text().collect())
}

fn get_urls(url: &mut url::Url, html: &str, artist_id: Uuid) -> Result<Vec<UrlData>> {
    let html = Html::parse_document(html);
    let mut urls = Vec::new();
    for anchor in html.select(&IMAGE_SELECTOR) {
        if let Some(abs_path) = anchor.value().attr("href") {
            url.set_path(abs_path);
            tracing::trace! {image_url = %url, artist = %artist_id, "Found artist image"};
            urls.push(img_url(url)?);
        }
    }
    Ok(urls)
}

async fn download<D>(
    db: &D,
    artist_id: Uuid,
    UrlData {
        image_page,
        image_url,
        image_id,
    }: UrlData,
) -> Result<Option<entity::Image>>
where
    D: ConnectionTrait,
{
    let library = &get_settings()?.library;
    let artist = entity::ArtistEntity::find_by_id(artist_id)
        .one(db)
        .await?
        .ok_or(eyre!("No arist with id {} found", artist_id))?;
    let artist_images_path = library
        .path
        .join(PathBuf::from_str(
            strfmt(
                library.artist_name.as_str(),
                &tag_to_string_map(&tags_from_artist(&artist)?),
            )?
            .as_str(),
        )?)
        .join(".images");
    mkdirp(&artist_images_path)?;
    let image_path = artist_images_path.join(image_id);

    if !image_path.exists() {
        let text = get_html(image_page).await?;
        let description = extract_description(text.as_str());

        let bytes = get(image_url.to_owned()).await?.bytes().await?;
        let format = infer::get(&bytes)
            .and_then(|t| t.mime_type().parse().ok())
            .and_then(ImageFormat::from_mime)
            .unwrap_or(ImageFormat::Jpeg);
        if format == ImageFormat::Gif {
            // TODO: properly handle animated images, including gifs
            return Ok(None);
        }
        let len = bytes.len();
        let img = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()?
            .decode()?;
        let path_str = path_to_str(&image_path)?;
        img.save_with_format(image_path, format.into())?;

        return Ok(Some(entity::Image {
            id: sha256::digest(path_str.as_str()),
            role: PictureType::Artist.to_string(),
            format,
            description,
            width: img.width() as i32,
            height: img.height() as i32,
            size: len as i32,
            path: path_str,
        }));
    } else {
        tracing::trace!(?image_path, "Image already exists, avoiding recloning");
    }

    Ok(None)
}

#[async_trait::async_trait]
impl super::TaskTrait for Data {
    async fn run<C>(&self, db: &C, _task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let Data(artist_id, url) = self;
        tracing::trace!(%artist_id, %url, "Fetching artist images from lastfm");
        let mut url = (url.clone() + "/").parse::<url::Url>()?.join("+images")?;
        let text = get_html(url.to_owned()).await?;

        // TODO: paginate over lastfm, setting limit
        let urls = get_urls(&mut url, text.as_str(), *artist_id)?;
        for url_data in urls.into_iter() {
            let new_image = download(&tx, *artist_id, url_data).await?;
            if let Some(image) = new_image {
                let id = image.id.to_owned();
                tracing::trace! {path = ?image.path, artist_id = %artist_id, "Downloaded image for artist"};
                entity::ImageEntity::insert(image.into_active_model())
                    .on_conflict(entity::conflict::IMAGE_CONFLICT_1.to_owned())
                    .on_conflict(entity::conflict::IMAGE_CONFLICT_2.to_owned())
                    .exec(&tx)
                    .await
                    .ignore_none()?;

                let artist_image = entity::ImageArtist {
                    image_id: id,
                    artist_id: *artist_id,
                };
                entity::ImageArtistEntity::insert(artist_image.into_active_model())
                    .on_conflict(entity::conflict::IMAGE_ARTIST_CONFLICT.to_owned())
                    .exec(&tx)
                    .await
                    .ignore_none()?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl super::TaskEntities for Data {
    async fn all<C>(db: &C) -> Result<Vec<Self>>
    where
        C: ConnectionTrait,
        Self: Sized,
    {
        Ok(entity::ArtistUrlEntity::find()
            .filter(entity::ArtistUrlColumn::Type.eq(entity::ArtistUrlType::LastFM))
            .all(db)
            .await?
            .into_iter()
            .map(|a| Self(a.artist_id, a.url))
            .collect())
    }

    async fn outdated<C>(db: &C) -> Result<Vec<Self>>
    where
        C: ConnectionTrait,
        Self: Sized,
    {
        let settings = get_settings()?;
        let before = time::OffsetDateTime::now_utc() - settings.tasks.outdated;

        Ok(entity::ArtistUrlEntity::find()
            .join(JoinType::LeftJoin, entity::ArtistRelation::Updates.def())
            .filter(
                entity::update_filter_condition(before, entity::UpdateType::LastFMArtistImage)
                    .and(entity::ArtistUrlColumn::Type.eq(entity::ArtistUrlType::LastFM)),
            )
            .all(db)
            .await?
            .into_iter()
            .map(|a| Self(a.artist_id, a.url))
            .collect())
    }
}
