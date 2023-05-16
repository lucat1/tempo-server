use eyre::{bail, eyre, Result, WrapErr};
use image::io::Reader as ImageReader;
use lazy_static::lazy_static;
use reqwest::{get, Method, Request};
use scraper::{Html, Selector};
use sea_orm::{ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter};
use std::str::FromStr;
use std::{io::Cursor, path::PathBuf};
use strfmt::strfmt;
use tag::PictureType;
use uuid::Uuid;

use crate::fetch::musicbrainz::send_request;
use base::setting::get_settings;
use base::util::{mkdirp, path_to_str};
use base::ImageFormat;
use entity::IgnoreNone;
use tag::{map::tags_from_artist, tag_to_string_map};

lazy_static! {
    static ref IMAGE_SELECTOR: Selector =
        Selector::parse("ul.image-list li.image-list-item-wrapper a.image-list-item").unwrap();
}
static LASTFM_IMAGE_ATTEMPT_SIZE: usize = 4096;

#[derive(Debug, Clone)]
pub struct Data(Uuid, String);

pub async fn all_data(db: &DbConn) -> Result<Vec<Data>> {
    Ok(entity::ArtistUrlEntity::find()
        .filter(entity::ArtistUrlColumn::Type.eq(entity::ArtistUrlType::LastFM))
        .all(db)
        .await?
        .into_iter()
        .map(|a| Data(a.artist_id, a.url))
        .collect())
}

fn img_url(url: &url::Url) -> Result<(url::Url, String)> {
    if let Some(path) = url.path_segments() {
        if let Some(id) = path.last() {
            let url = format!(
                "https://lastfm.freetls.fastly.net/i/u/{}x0/{}.jpg#{}",
                LASTFM_IMAGE_ATTEMPT_SIZE, id, id
            )
            .parse()?;
            return Ok((url, id.to_string()));
        }
    }

    Err(eyre!(
        "Could not extract image id from last.fm url: {}",
        url
    ))
}

async fn download(
    db: &DbConn,
    artist_id: Uuid,
    url: &url::Url,
    id: String,
) -> Result<Option<entity::Image>> {
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
    let image_path = artist_images_path.join(id);

    if !image_path.exists() {
        let bytes = get(url.to_owned()).await?.bytes().await?;
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
            // TODO: actually fetch description from last.fm
            description: None,
            width: img.width(),
            height: img.height(),
            size: len as u32,
            path: path_str,
        }));
    } else {
        tracing::trace!(?image_path, "Image already exists, avoiding recloning");
    }

    Ok(None)
}

fn get_urls(url: &mut url::Url, text: &String, artist_id: Uuid) -> Result<Vec<(url::Url, String)>> {
    let html = Html::parse_document(text.as_str());
    let mut urls = Vec::new();
    for anchor in html.select(&IMAGE_SELECTOR) {
        if let Some(abs_path) = anchor.value().attr("href") {
            url.set_path(abs_path);
            tracing::trace! {image_url = %url, artist = %artist_id, "Found artist image"};
            urls.push(img_url(&url)?);
        }
    }
    Ok(urls)
}

pub async fn run(db: &DbConn, Data(artist_id, url): Data) -> Result<()> {
    tracing::trace!(%artist_id, %url, "Fetching artist images from lastfm");
    let mut url = (url.clone() + "/").parse::<url::Url>()?.join("+images")?;
    let res = send_request(Request::new(Method::GET, url.to_owned())).await?;
    if !res.status().is_success() {
        bail!(
            "Last.fm request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let text = res
        .text()
        .await
        .wrap_err(eyre!("Could not read response as text"))?;

    // TODO: paginate over lastfm, setting limit
    let urls = get_urls(&mut url, &text, artist_id)?;
    for (image_url, image_id) in urls.into_iter() {
        let new_image = download(db, artist_id, &image_url, image_id.clone()).await?;
        if let Some(image) = new_image {
            let id = image.id.to_owned();
            tracing::trace! {path = ?image.path, %image_id, artist_id = %artist_id, "Downloaded image for artist"};
            entity::ImageEntity::insert(image.into_active_model())
                .on_conflict(entity::conflict::IMAGE_CONFLICT_1.to_owned())
                .on_conflict(entity::conflict::IMAGE_CONFLICT_2.to_owned())
                .exec(db)
                .await
                .ignore_none()?;

            let artist_image = entity::ImageArtist {
                image_id: id,
                artist_id,
            };
            entity::ImageArtistEntity::insert(artist_image.into_active_model())
                .on_conflict(entity::conflict::IMAGE_ARTIST_CONFLICT.to_owned())
                .exec(db)
                .await
                .ignore_none()?;
        }
    }
    Ok(())
}
