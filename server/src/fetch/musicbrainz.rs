use const_format::formatcp;
use governor::{clock::*, middleware::*, state::*, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero_ext::*;
use reqwest::{header::USER_AGENT, Error, Request, Response};
use std::num::NonZeroU32;

pub static MB_BASE_URL: &str = "https://musicbrainz.org/ws/2/";
static MB_CALLS_PER_SECOND: NonZeroU32 = nonzero!(1u32);
static MB_USER_AGENT: &str = formatcp!("{}/{} ({})", base::CLI_NAME, base::VERSION, base::GITHUB);

lazy_static! {
    static ref UNLIMITED_CLIENT: reqwest::Client = reqwest::Client::new();
    static ref LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware> =
        RateLimiter::direct(Quota::per_second(MB_CALLS_PER_SECOND));
}

pub async fn send_request(mut req: Request) -> Result<Response, Error> {
    LIMITER.until_ready().await;
    let headers = req.headers_mut();
    headers.append(USER_AGENT, MB_USER_AGENT.parse().unwrap());
    UNLIMITED_CLIENT.execute(req).await
}
