use eyre::{bail, eyre, Result, WrapErr};
use futures::StreamExt;
use lazy_static::lazy_static;
use reqwest::{get, Method, Request};
use scraper::{Html, Selector};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter,
};
use std::path::PathBuf;
use std::str::FromStr;
use strfmt::strfmt;
use uuid::Uuid;

use crate::fetch::musicbrainz::send_request;
use base::setting::get_settings;
use base::util::mkdirp;
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

async fn download(db: &DbConn, artist_id: Uuid, url: &url::Url, id: String) -> Result<PathBuf> {
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
    mkdirp(&artist_images_path);
    let image_path = artist_images_path.join(id.to_string() + ".jpg");

    if !image_path.exists() {
        let mut file = tokio::fs::File::create(image_path.clone()).await?;
        let mut response = get(url.to_owned()).await?.bytes_stream();
        while let Some(item) = response.next().await {
            tokio::io::copy(&mut item?.as_ref(), &mut file).await?;
        }
    } else {
        tracing::trace!(?image_path, "Image already exists, avoiding recloning");
    }

    Ok(image_path)
}

pub async fn run(db: &DbConn, Data(id, url): Data) -> Result<()> {
    tracing::trace!(%id, %url, "Fetching artist images from lastfm");
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
    let html = Html::parse_document(text.as_str());
    let mut urls = Vec::new();
    for anchor in html.select(&IMAGE_SELECTOR) {
        if let Some(abs_path) = anchor.value().attr("href") {
            url.set_path(abs_path);
            tracing::trace! {image_url = %url, artist = %id, "Found artist image"};
            urls.push(img_url(&url)?);
        }
    }
    for (image_url, image_id) in urls.into_iter() {
        let path = download(db, id, &image_url, image_id.clone()).await?;
        // tracing::trace! {?path, %image_id, artist_id = %id, "Downloaded image for artist"};
    }
    Ok(())
}
