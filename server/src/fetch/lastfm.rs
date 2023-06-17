use governor::{clock::*, middleware::*, state::*, Quota, RateLimiter};
use lazy_static::lazy_static;
use md5::compute as md5;
use nonzero_ext::*;
use reqwest::{header::HeaderValue, header::USER_AGENT, Error, Request, Response};
use std::num::NonZeroU32;

pub static LASTFM_BASE_STRURL: &str = "https://ws.audioscrobbler.com/2.0/?format=json";
static LASTFM_CALLS_PER_SECOND: NonZeroU32 = nonzero!(1u32);

lazy_static! {
    pub static ref LASTFM_BASE_URL: url::Url = url::Url::parse(LASTFM_BASE_STRURL).unwrap();
    static ref UNLIMITED_CLIENT: reqwest::Client = reqwest::Client::new();
    static ref LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware> =
        RateLimiter::direct(Quota::per_second(LASTFM_CALLS_PER_SECOND));
    static ref LASTFM_USER_AGENT: HeaderValue =
        "Mozilla/5.0 (Android 4.4; Mobile; rv:41.0) Gecko/41.0 Firefox/41.0"
            .parse()
            .unwrap();
}

pub async fn send_request(mut req: Request) -> Result<Response, Error> {
    LIMITER.until_ready().await;
    let headers = req.headers_mut();
    headers.append(USER_AGENT, LASTFM_USER_AGENT.clone());
    UNLIMITED_CLIENT.execute(req).await
}

pub fn signature<I, T>(pairs: I, secret: &str) -> String

where T: Into<String>, I: Iterator<Item = (T, T)>
{
    let mut sorted_pairs: Vec<(String, String)> = pairs
        .map(|(k,v)| (k.into(), v.into()))
        .filter(|(k, _)| k != "format")
        .collect();
    sorted_pairs.sort_by_key(|(k, _)| k.to_owned());
    let concat = sorted_pairs
        .iter()
        .fold(String::new(), |concat, (key, value)| concat + key + value)
        + secret;
    let hex = format!("{:x}", md5(concat.as_bytes()));
    tracing::debug!(signature = ?concat, md5 = ?hex, "LastFM signature");
    hex
}
