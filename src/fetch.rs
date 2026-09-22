//! HTTP document retrieval boundary.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use reqwest::{StatusCode, blocking::Client, redirect::Policy};
use url::Url;

/// Settings applied to every HTTP(S) document request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchConfiguration {
    user_agent: String,
    timeout: Duration,
    max_retries: usize,
}

impl FetchConfiguration {
    /// Creates request settings with a configurable identifying user agent.
    pub fn new(user_agent: impl Into<String>, timeout: Duration, max_retries: usize) -> Self {
        Self {
            user_agent: user_agent.into(),
            timeout,
            max_retries,
        }
    }
}

impl Default for FetchConfiguration {
    fn default() -> Self {
        Self::new("rust-skgen/0.1", Duration::from_secs(30), 2)
    }
}

/// An HTTP response retrieved by a document fetcher.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchedDocument {
    url: Url,
    status: u16,
    body: String,
    content_type: Option<String>,
}

impl FetchedDocument {
    /// Creates a fetched document from its response details.
    pub fn new(url: Url, status: u16, body: String) -> Self {
        Self {
            url,
            status,
            body,
            content_type: None,
        }
    }

    /// Creates a fetched document with its response content type.
    pub fn new_with_content_type(
        url: Url,
        status: u16,
        body: String,
        content_type: String,
    ) -> Self {
        Self {
            url,
            status,
            body,
            content_type: Some(content_type),
        }
    }

    /// Returns the URL that produced this response.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns the HTTP response status code.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns the response body decoded by the HTTP client.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns whether the response content type is suitable for HTML extraction.
    pub fn is_html(&self) -> bool {
        self.content_type.as_deref().is_none_or(|content_type| {
            matches!(
                content_type.split(';').next().map(str::trim),
                Some("text/html" | "application/xhtml+xml")
            )
        })
    }

    /// Returns whether the response content type identifies XML.
    pub fn is_xml(&self) -> bool {
        self.content_type.as_deref().is_some_and(|content_type| {
            let media_type = content_type
                .split(';')
                .next()
                .map(str::trim)
                .unwrap_or_default();
            matches!(media_type, "application/xml" | "text/xml") || media_type.ends_with("+xml")
        })
    }

    /// Returns the response content type when the fetcher provided it.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
}

/// Error returned while configuring or retrieving an HTTP document.
#[derive(Debug)]
pub enum FetchError {
    /// The HTTP client could not be configured.
    Client(reqwest::Error),
    /// The request did not receive a response after its permitted attempts.
    Request(reqwest::Error),
    /// The response body could not be read.
    ResponseBody(reqwest::Error),
    /// The shared sequential request lock was poisoned.
    RequestLockPoisoned,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client(error) => write!(formatter, "failed to configure HTTP client: {error}"),
            Self::Request(error) => write!(formatter, "failed to retrieve HTTP document: {error}"),
            Self::ResponseBody(error) => {
                write!(formatter, "failed to read HTTP response body: {error}")
            }
            Self::RequestLockPoisoned => formatter.write_str("HTTP request lock was poisoned"),
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Client(error) | Self::Request(error) | Self::ResponseBody(error) => Some(error),
            Self::RequestLockPoisoned => None,
        }
    }
}

/// Retrieves source documents over HTTP(S).
pub trait DocumentFetcher {
    /// Fetches one document without following redirects.
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError>;
}

/// A blocking HTTP(S) fetcher that serializes all requests shared by its clones.
#[derive(Clone)]
pub struct HttpDocumentFetcher {
    client: Client,
    max_retries: usize,
    request_lock: Arc<Mutex<()>>,
}

impl HttpDocumentFetcher {
    /// Builds a fetcher that identifies requests, times them out, and disables redirects.
    pub fn new(configuration: FetchConfiguration) -> Result<Self, FetchError> {
        let client = Client::builder()
            .user_agent(configuration.user_agent)
            .timeout(configuration.timeout)
            .redirect(Policy::none())
            .build()
            .map_err(FetchError::Client)?;

        Ok(Self {
            client,
            max_retries: configuration.max_retries,
            request_lock: Arc::new(Mutex::new(())),
        })
    }

    fn should_retry(status: StatusCode) -> bool {
        status == StatusCode::REQUEST_TIMEOUT
            || status == StatusCode::TOO_MANY_REQUESTS
            || status.is_server_error()
    }
}

impl DocumentFetcher for HttpDocumentFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        // Holding this lock through retries enforces one in-flight request per fetcher family.
        let _request_lock = self
            .request_lock
            .lock()
            .map_err(|_| FetchError::RequestLockPoisoned)?;
        let mut retries_remaining = self.max_retries;

        loop {
            match self.client.get(url.clone()).send() {
                Ok(response) if Self::should_retry(response.status()) && retries_remaining > 0 => {
                    retries_remaining -= 1;
                }
                Ok(response) => {
                    let document_url = response.url().clone();
                    let status = response.status().as_u16();
                    let content_type = response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_owned);
                    let body = response.text().map_err(FetchError::ResponseBody)?;
                    return Ok(match content_type {
                        Some(content_type) => FetchedDocument::new_with_content_type(
                            document_url,
                            status,
                            body,
                            content_type,
                        ),
                        None => FetchedDocument::new(document_url, status, body),
                    });
                }
                Err(_) if retries_remaining > 0 => {
                    retries_remaining -= 1;
                }
                Err(error) => return Err(FetchError::Request(error)),
            }
        }
    }
}
