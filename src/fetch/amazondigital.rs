use eyre::{bail, eyre, Result};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::info;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Url;
use scraper::{Html, Selector};
use setting::ArtProvider;
use setting::Settings;
use std::time::Instant;

use super::{cover::probe, Cover, CLIENT};
use entity::full::ArtistInfo;
use entity::full::FullRelease;

struct AmazonImageFormat(usize, usize, usize);

lazy_static! {
    static ref TITLE_SELECTOR: Selector = Selector::parse("head > title").unwrap();
    static ref TITLE_BLOCKED: Vec<&'static str> =
        vec!["Robot Check", "Bot Check", "Amazon CAPTCHA"];
    static ref LINK_SELECTOR: Selector = Selector::parse("a").unwrap();
    static ref RESULT_AND_IMAGE_SELECTORS: Vec<(Selector, Selector)> = vec![
        (
            Selector::parse("span.rush-component[data-component-type='s-product-image']").unwrap(),
            Selector::parse("img.s-image").unwrap(),
        ),
        (
            Selector::parse("div#dm_mp3Player li.s-mp3-federated-bar-item").unwrap(),
            Selector::parse("img.s-access-image").unwrap()
        )
    ];
    static ref IMAGE_FORMATS: Vec<AmazonImageFormat> = vec![
        AmazonImageFormat(0, 1, 600),
        AmazonImageFormat(1, 2, 700),
        AmazonImageFormat(1, 4, 1280),
        AmazonImageFormat(2, 3, 1025),
        AmazonImageFormat(2, 5, 1920),
        AmazonImageFormat(3, 4, 1500),
        AmazonImageFormat(3, 7, 2560),
    ];
    static ref HEADERS: HeaderMap = [
        (
            HeaderName::from_static("user-agent"),
            HeaderValue::from_str(
                "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0"
            )
            .unwrap()
        ),
        (
            HeaderName::from_static("accept"),
            HeaderValue::from_str(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
            )
            .unwrap()
        ),
        (
            HeaderName::from_static("accept-language"),
            HeaderValue::from_str("en-US,en;q=0.9").unwrap()
        ),
        (
            HeaderName::from_static("dnt"),
            HeaderValue::from_str("1").unwrap()
        ),
        (
            HeaderName::from_static("connection"),
            HeaderValue::from_str("Keep-Alive").unwrap()
        ),
        (
            HeaderName::from_static("upgrade-insecure-requests"),
            HeaderValue::from_str("1").unwrap()
        ),
        (
            HeaderName::from_static("cache-control"),
            HeaderValue::from_str("max-age=0").unwrap()
        ),
        (
            HeaderName::from_static("te"),
            HeaderValue::from_str("trailers").unwrap()
        )
    ]
    .iter()
    .cloned()
    .collect();
    static ref API_KEY: &'static str = "A17SFUTIVB227Z";
}

fn make_cover(urls: Vec<String>, size: usize, title: &str, artist: &str) -> Cover {
    Cover {
        provider: ArtProvider::AmazonDigital,
        urls,
        width: size,
        height: size,
        title: title.to_string(),
        artist: artist.to_string(),
    }
}

pub async fn fetch_amazondigital(full_release: &FullRelease, _: &Settings) -> Result<Vec<Cover>> {
    let mut start = Instant::now();
    let mut covers = Vec::new();
    let (artists, title) = (
        full_release.get_joined_artists()?,
        full_release.release.title.clone(),
    );

    info!("data {:?}", HEADERS.clone());
    let res = CLIENT
        .get(format!(
            "https://www.amazon.com/s?i=digital-music&s=relevancerank&k={}",
            artists.clone() + " " + title.as_str()
        ))
        .headers(HEADERS.clone())
        .send()
        .await?;
    info!(
        "Amazon digital HTTP request took {:?}, code {}",
        start.elapsed(),
        res.status()
    );
    start = Instant::now();
    if !res.status().is_success() {
        bail!(
            "Amazon digital request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let html = Html::parse_document(res.text().await?.as_str());
    info!("{}", html.html());
    let title_str = html
        .select(&TITLE_SELECTOR)
        .next()
        .map(|title| title.text().collect::<Vec<_>>().join(""))
        .unwrap_or_default();
    if TITLE_BLOCKED.iter().any(|s| title_str.contains(s)) {
        bail!("Amazon blocked the request");
    }

    for (result_selector, image_selector) in RESULT_AND_IMAGE_SELECTORS.iter() {
        if let Some(thumbnail_url) = html
            .select(result_selector)
            .next()
            .and_then(|result| result.select(image_selector).next())
            .and_then(|image| image.value().attr("src"))
        {
            let thumb = thumbnail_url.replace("Stripe-Prime-Only", "");
            let img_url = thumb.rsplit(".").take(2).join(".");
            covers.push(make_cover(
                vec![img_url],
                500,
                title.as_str(),
                artists.as_str(),
            ));
        }
    }
    if let Some(product_url) = html
        .select(&LINK_SELECTOR)
        .next()
        .and_then(|image| image.value().attr("href"))
        .and_then(|product_url| Url::parse(product_url).ok())
    {
        if let Some(product_id) = product_url
            .path_segments()
            .and_then(|segments| segments.step_by(3).next())
        {
            for AmazonImageFormat(id, per_side, size) in IMAGE_FORMATS.iter() {
                let mut urls = vec![];
                for x in 0..*per_side {
                    for y in 0..*per_side {
                        urls.push(
                            "https://z2-ec2.images-amazon.com/R/1/a=".to_string()
                                + product_id
                                + "+c="
                                + &API_KEY
                                + "+d=_SCR%28"
                                + id.to_string().as_str()
                                + ","
                                + x.to_string().as_str()
                                + ","
                                + y.to_string().as_str()
                                + "%29_=.jpg",
                        )
                    }
                }
                info!(
                    "Amazon digital trying url (size: {}): {}",
                    size,
                    urls[urls.len() - 1]
                );
                info!("URLS {:?}", urls);
                if probe(urls[urls.len() - 1].as_str(), None).await.is_some() {
                    covers.push(make_cover(urls, *size, title.as_str(), artists.as_str()));
                };
            }
        }
    }
    info!("Amazon digital HTML parse took {:?}", start.elapsed());
    Ok(covers)
}
