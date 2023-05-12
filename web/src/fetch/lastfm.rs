use axum::http::HeaderValue;
use governor::{clock::*, middleware::*, state::*, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero_ext::*;
use reqwest::{header::USER_AGENT, Error, Request, Response};
use std::num::NonZeroU32;

static LASTFM_CALLS_PER_SECOND: NonZeroU32 = nonzero!(1u32);

lazy_static! {
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
