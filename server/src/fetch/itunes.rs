use governor::{clock::*, middleware::*, state::*, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero_ext::*;
use reqwest::{header::HeaderValue, header::USER_AGENT, Error, Request, Response};
use std::num::NonZeroU32;

pub static ITUNES_BASE_STRURL: &str = "http://itunes.apple.com/";
static ITUNES_CALLS_PER_SECOND: NonZeroU32 = nonzero!(10u32);

lazy_static! {
    pub static ref ITUNES_BASE_URL: url::Url = url::Url::parse(ITUNES_BASE_STRURL).unwrap();
    static ref UNLIMITED_CLIENT: reqwest::Client = reqwest::Client::new();
    static ref LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware> =
        RateLimiter::direct(Quota::per_second(ITUNES_CALLS_PER_SECOND));
}

pub async fn send_request(mut req: Request) -> Result<Response, Error> {
    LIMITER.until_ready().await;
    let headers = req.headers_mut();
    UNLIMITED_CLIENT.execute(req).await
}
