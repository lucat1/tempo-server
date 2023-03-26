use eyre::{bail, Result};
use log::trace;
use serde_derive::{Deserialize, Serialize};
use std::time::Instant;

use super::{cover::probe, Cover, CLIENT};
use base::setting::ArtProvider;
use entity::full::{ArtistInfo, FullRelease};

static DEFAULT_COUNTRY: &str = "US";
static ITUNES_COUNTRIES: &[&str] = &[
    "AE", "AG", "AI", "AL", "AM", "AO", "AR", "AT", "AU", "AZ", "BB", "BE", "BF", "BG", "BH", "BJ",
    "BM", "BN", "BO", "BR", "BS", "BT", "BW", "BY", "BZ", "CA", "CG", "CH", "CL", "CN", "CO", "CR",
    "CV", "CY", "CZ", "DE", "DK", "DM", "DO", "DZ", "EC", "EE", "EG", "ES", "FI", "FJ", "FM", "FR",
    "GB", "GD", "GH", "GM", "GR", "GT", "GW", "GY", "HK", "HN", "HR", "HU", "ID", "IE", "IL", "IN",
    "IS", "IT", "JM", "JO", "JP", "KE", "KG", "KH", "KN", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LK", "LR", "LT", "LU", "LV", "MD", "MG", "MK", "ML", "MN", "MO", "MR", "MS", "MT", "MU", "MW",
    "MX", "MY", "MZ", "NA", "NE", "NG", "NI", "NL", "NP", "NO", "NZ", "OM", "PA", "PE", "PG", "PH",
    "PK", "PL", "PT", "PW", "PY", "QA", "RO", "RU", "SA", "SB", "SC", "SE", "SG", "SI", "SK", "SL",
    "SN", "SR", "ST", "SV", "SZ", "TC", "TD", "TH", "TJ", "TM", "TN", "TR", "TT", "TW", "TZ", "UA",
    "UG", "US", "UY", "UZ", "VC", "VE", "VG", "VN", "YE", "ZA", "ZW",
];

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Itunes {
    pub results: Vec<ItunesResult>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItunesResult {
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(rename = "artworkUrl100")]
    pub artwork_url_100: String,
}

pub async fn fetch(full_release: &FullRelease) -> Result<Vec<Cover>> {
    let FullRelease { release, .. } = full_release;
    let start = Instant::now();
    let raw_country = release
        .country
        .clone()
        .unwrap_or(DEFAULT_COUNTRY.to_string());
    let country = if ITUNES_COUNTRIES.contains(&raw_country.as_str()) {
        raw_country.as_str()
    } else {
        DEFAULT_COUNTRY
    };

    let res = CLIENT
        .get(format!(
            "http://itunes.apple.com/search?media=music&entity=album&country={}&term={}",
            country,
            full_release.get_joined_artists()? + " " + release.title.clone().as_str()
        ))
        .send()
        .await?;
    let req_time = start.elapsed();
    trace!("Itunes HTTP request took {:?}", req_time);
    if !res.status().is_success() {
        bail!(
            "Itunes request returned non-success error code: {} {}",
            res.status(),
            res.text().await?
        );
    }
    let json = res.json::<Itunes>().await?;
    let mut res = vec![];
    for result in json.results.iter() {
        for size in [5000, 1200, 600].iter() {
            let url = result
                .artwork_url_100
                .replace("100x100", format!("{size}x{size}").as_str());
            if !probe(url.as_str(), None).await {
                continue;
            }
            res.push(Cover {
                provider: ArtProvider::Itunes,
                url,
                width: *size,
                height: *size,
                title: result.collection_name.clone(),
                artist: result.artist_name.clone(),
            })
        }
    }
    Ok(res)
}
