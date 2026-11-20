//! The server's `/runner/*` routes, as a runner calls them.

use std::fmt;

use antilog_types::{Item, Items, PollRequest, PollResponse, Release, RunResult, SessionId};
use anyhow::{Context, Result, bail};
use reqwest::{Client, StatusCode};

/// The server says this runner no longer holds the session: stop it.
#[derive(Debug)]
pub struct NotHeld;

impl fmt::Display for NotHeld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the server says this runner no longer holds the session")
    }
}

impl std::error::Error for NotHeld {}

#[derive(Clone)]
pub struct Server {
    http: Client,
    base: String,
}

impl Server {
    pub fn new(base: &str) -> Self {
        Self {
            http: Client::new(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    pub async fn poll(&self, key: &str, request: &PollRequest) -> Result<PollResponse> {
        let response = self
            .http
            .post(self.url("/runner/poll"))
            .bearer_auth(key)
            .json(request)
            .send()
            .await
            .context("polling the server")?;
        if !response.status().is_success() {
            bail!("the server refused the poll: {}", response.status());
        }
        response.json().await.context("reading the poll answer")
    }

    /// A call about one session, with that session's runner token.
    async fn about(
        &self,
        session: SessionId,
        token: &str,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<()> {
        let response = self
            .http
            .post(self.url(&format!("/runner/sessions/{session}/{path}")))
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .with_context(|| format!("sending {path}"))?;
        match response.status() {
            status if status.is_success() => Ok(()),
            StatusCode::CONFLICT | StatusCode::UNAUTHORIZED => Err(NotHeld.into()),
            status => bail!("the server refused {path}: {status}"),
        }
    }

    pub async fn items(&self, session: SessionId, token: &str, items: &[Item]) -> Result<()> {
        let body = Items {
            items: items.to_vec(),
        };
        self.about(session, token, "events", &body).await
    }

    pub async fn result(&self, session: SessionId, token: &str, result: &RunResult) -> Result<()> {
        self.about(session, token, "results", result).await
    }

    pub async fn release(&self, session: SessionId, token: &str, release: &Release) -> Result<()> {
        self.about(session, token, "release", release).await
    }

    async fn download(&self, path: &str, token: &str) -> Result<Option<reqwest::Response>> {
        let response = self
            .http
            .get(self.url(path))
            .bearer_auth(token)
            .send()
            .await
            .with_context(|| format!("fetching {path}"))?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(None),
            status if status.is_success() => Ok(Some(response)),
            status => bail!("the server refused {path}: {status}"),
        }
    }

    /// The export an earlier run left, if any.
    pub async fn export(&self, session: SessionId, token: &str) -> Result<Option<String>> {
        match self
            .download(&format!("/runner/sessions/{session}/export"), token)
            .await?
        {
            Some(response) => Ok(Some(response.text().await?)),
            None => Ok(None),
        }
    }

    /// A skill pack; `path` is what the spec gave.
    pub async fn skill(&self, path: &str, token: &str) -> Result<Option<Vec<u8>>> {
        match self.download(path, token).await? {
            Some(response) => Ok(Some(response.bytes().await?.to_vec())),
            None => Ok(None),
        }
    }
}
