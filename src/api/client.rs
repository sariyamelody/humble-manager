use anyhow::Result;
use reqwest::{Client, header::{HeaderMap, HeaderValue}};

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
            .build()?;

        Ok(Self { client })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
