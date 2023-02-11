use eyre::{bail, Result};
use itertools::Itertools;
use log::trace;
use serde_derive::{Deserialize, Serialize};
use std::time::Instant;

use super::{cover::probe, Cover, CLIENT};
use entity::full::FullRelease;
use setting::ArtProvider;

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
    pub max_size: Option<usize>,
}

impl From<Itunes> for Vec<super::cover::Cover> {
    fn from(caa: Itunes) -> Self {
        caa.results
            .into_iter()
            .filter_map(|i| {
                i.max_size.map(|s| super::cover::Cover {
                    provider: ArtProvider::Itunes,
                    urls: vec![i
                        .artwork_url_100
                        .replace("100x100", format!("{}x{}", s, s).as_str())],
                    width: s,
                    height: s,
                    title: i.collection_name,
                    artist: i.artist_name,
                })
            })
            .collect()
    }
}

pub async fn fetch(release: &FullRelease) -> Result<Vec<Cover>> {
    let FullRelease {
        release, artist, ..
    } = release;
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

    // TODO: make "," configurable
    let res = CLIENT
        .get(format!(
            "http://itunes.apple.com/search?media=music&entity=album&country={}&term={}",
            country,
            artist.into_iter().map(|a| a.name.clone()).join(",")
                + " "
                + release.title.clone().as_str()
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
    let mut json = res.json::<Itunes>().await?;
    for item in json.results.iter_mut() {
        for size in [5000, 1200, 600] {
            let url = item
                .artwork_url_100
                .replace("100x100", format!("{}x{}", size, size).as_str());
            if probe(url.as_str(), None).await {
                item.max_size = Some(size);
                break;
            } else {
                continue;
            }
        }
    }
    let json_time = start.elapsed();
    trace!("Itunes JSON parse took {:?}", json_time - req_time);
    Ok(json.into())
}
