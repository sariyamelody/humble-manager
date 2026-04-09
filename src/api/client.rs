use anyhow::Result;
use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use std::time::Duration;

pub struct HumbleClient {
    client: Client,
}

impl HumbleClient {
    pub fn new(session_cookie: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("X-Requested-By", HeaderValue::from_static("hb_android_app"));

        let cookie = format!("_simpleauth_sess={}", session_cookie);
        headers.insert(
            reqwest::header::COOKIE,
            HeaderValue::from_str(&cookie)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .gzip(true)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { client })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
