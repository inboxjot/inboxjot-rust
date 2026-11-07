use std::fmt;
use std::fmt::Write as _;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, RETRY_AFTER, USER_AGENT};
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::campaigns::Campaigns;
use crate::contacts::Contacts;
use crate::domains::Domains;
use crate::emails::Emails;
use crate::error::{ApiError, Error, ErrorName, Result};
use crate::events::Events;
use crate::images::Images;
use crate::segments::Segments;
use crate::templates::Templates;
use crate::topics::Topics;
use crate::webhook_endpoints::Webhooks;

const DEFAULT_BASE_URL: &str = "https://api.inboxjot.com";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_RETRIES: u32 = 2;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);
const NO_QUERY: Option<&()> = None;
pub(crate) const NO_BODY: Option<&()> = None;
const SDK_USER_AGENT: &str = concat!("inboxjot-rust/", env!("CARGO_PKG_VERSION"));

#[derive(Clone)]
pub struct Inboxjot {
  http: reqwest::Client,
  api_key: String,
  base_url: String,
  timeout: Duration,
  max_retries: u32,
}

impl fmt::Debug for Inboxjot {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("Inboxjot")
      .field("base_url", &self.base_url)
      .field("timeout", &self.timeout)
      .field("max_retries", &self.max_retries)
      .finish_non_exhaustive()
  }
}

impl Inboxjot {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self::builder(api_key).build()
  }

  pub fn from_env() -> Result<Self> {
    let key = std::env::var("INBOXJOT_API_KEY")
      .ok()
      .filter(|key| !key.is_empty())
      .ok_or(Error::MissingApiKey)?;

    Ok(Self::new(key))
  }

  pub fn builder(api_key: impl Into<String>) -> InboxjotBuilder {
    InboxjotBuilder {
      api_key: api_key.into(),
      base_url: DEFAULT_BASE_URL.to_owned(),
      timeout: DEFAULT_TIMEOUT,
      max_retries: DEFAULT_MAX_RETRIES,
      http: None,
    }
  }

  pub fn emails(&self) -> Emails<'_> {
    Emails::new(self)
  }

  pub fn contacts(&self) -> Contacts<'_> {
    Contacts::new(self)
  }

  pub fn events(&self) -> Events<'_> {
    Events::new(self)
  }

  pub fn campaigns(&self) -> Campaigns<'_> {
    Campaigns::new(self)
  }

  pub fn templates(&self) -> Templates<'_> {
    Templates::new(self)
  }

  pub fn domains(&self) -> Domains<'_> {
    Domains::new(self)
  }

  pub fn images(&self) -> Images<'_> {
    Images::new(self)
  }

  pub fn segments(&self) -> Segments<'_> {
    Segments::new(self)
  }

  pub fn topics(&self) -> Topics<'_> {
    Topics::new(self)
  }

  pub fn webhooks(&self) -> Webhooks<'_> {
    Webhooks::new(self)
  }

  pub(crate) async fn send<T: DeserializeOwned>(
    &self,
    method: Method,
    segments: &[&str],
    body: Option<&(impl Serialize + ?Sized)>,
    idempotency_key: Option<&str>,
  ) -> Result<T> {
    self
      .send_with_query(method, segments, NO_QUERY, body, idempotency_key)
      .await
  }

  pub(crate) async fn send_with_query<T: DeserializeOwned>(
    &self,
    method: Method,
    segments: &[&str],
    query: Option<&(impl Serialize + ?Sized)>,
    body: Option<&(impl Serialize + ?Sized)>,
    idempotency_key: Option<&str>,
  ) -> Result<T> {
    let url = build_url(&self.base_url, segments);
    let body = body.map(serde_json::to_vec).transpose()?;
    let is_safe =
      method == Method::GET || method == Method::DELETE || idempotency_key.is_some();

    let mut attempt = 0;
    loop {
      let authorization = format!("Bearer {}", self.api_key);
      let mut request = self
        .http
        .request(method.clone(), &url)
        .timeout(self.timeout)
        .header(AUTHORIZATION, authorization)
        .header(USER_AGENT, SDK_USER_AGENT);

      if let Some(query) = query {
        request = request.query(query);
      }

      if let Some(key) = idempotency_key {
        request = request.header("idempotency-key", key);
      }

      if let Some(body) = &body {
        request = request.header(CONTENT_TYPE, "application/json").body(body.clone());
      }

      let can_retry = attempt < self.max_retries;
      let response = match request.send().await {
        Ok(response) => response,
        Err(error) if can_retry && (error.is_connect() || is_safe) => {
          tokio::time::sleep(get_backoff(attempt)).await;
          attempt += 1;
          continue;
        }
        Err(error) => return Err(error.into()),
      };

      let status = response.status();
      if status.is_success() {
        let bytes = response.bytes().await?;
        return Ok(serde_json::from_slice(&bytes)?);
      }

      let retry_after = read_retry_after(response.headers());
      let error_body = response.bytes().await?;
      let error = read_api_error(status, &error_body);

      let is_rate_limited = status == StatusCode::TOO_MANY_REQUESTS;
      let is_concurrent = error.name == ErrorName::ConcurrentIdempotentRequests;
      let is_safe_server_error = status.is_server_error() && is_safe;
      let is_retryable = is_rate_limited || is_concurrent || is_safe_server_error;

      let delay = retry_after.unwrap_or_else(|| get_backoff(attempt));
      if !can_retry || !is_retryable || delay > MAX_RETRY_AFTER {
        return Err(error.into());
      }

      tokio::time::sleep(delay).await;
      attempt += 1;
    }
  }

  pub(crate) async fn put_upload(
    &self,
    url: &str,
    body: Vec<u8>,
    content_type: &str,
  ) -> Result<()> {
    let response = self
      .http
      .put(url)
      .timeout(self.timeout)
      .header(CONTENT_TYPE, content_type)
      .body(body)
      .send()
      .await?;

    let status = response.status();
    if status.is_success() {
      return Ok(());
    }

    let error_body = response.bytes().await?;
    let mut error = read_api_error(status, &error_body);
    error.status = status.as_u16();
    error.message = format!("The image upload failed with status {}", status.as_u16());

    Err(error.into())
  }
}

pub struct InboxjotBuilder {
  api_key: String,
  base_url: String,
  timeout: Duration,
  max_retries: u32,
  http: Option<reqwest::Client>,
}

impl fmt::Debug for InboxjotBuilder {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("InboxjotBuilder")
      .field("base_url", &self.base_url)
      .field("timeout", &self.timeout)
      .field("max_retries", &self.max_retries)
      .finish_non_exhaustive()
  }
}

impl InboxjotBuilder {
  pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
    self.base_url = base_url.into().trim_end_matches('/').to_owned();
    self
  }

  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  pub fn max_retries(mut self, max_retries: u32) -> Self {
    self.max_retries = max_retries;
    self
  }

  pub fn http_client(mut self, http: reqwest::Client) -> Self {
    self.http = Some(http);
    self
  }

  pub fn build(self) -> Inboxjot {
    Inboxjot {
      http: self.http.unwrap_or_default(),
      api_key: self.api_key,
      base_url: self.base_url,
      timeout: self.timeout,
      max_retries: self.max_retries,
    }
  }
}

fn build_url(base_url: &str, segments: &[&str]) -> String {
  let mut url = base_url.to_owned();

  for segment in segments {
    url.push('/');

    for byte in segment.bytes() {
      let is_unreserved = byte.is_ascii_alphanumeric() || b"-._~@".contains(&byte);
      if is_unreserved {
        url.push(char::from(byte));
      } else {
        let _ = write!(url, "%{byte:02X}");
      }
    }
  }

  url
}

fn get_backoff(attempt: u32) -> Duration {
  let backoff = 500 * 2_u64.pow(attempt.min(4));
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |elapsed| elapsed.subsec_nanos());
  let jitter = u64::from(nanos) % (backoff / 4);

  Duration::from_millis(backoff - jitter)
}

fn read_retry_after(headers: &HeaderMap) -> Option<Duration> {
  let value = headers.get(RETRY_AFTER)?.to_str().ok()?;
  let seconds = value.trim().parse().ok()?;

  Some(Duration::from_secs(seconds))
}

fn read_api_error(status: StatusCode, body: &[u8]) -> ApiError {
  if let Ok(error) = serde_json::from_slice(body) {
    return error;
  }

  let message = String::from_utf8_lossy(body).trim().to_owned();
  ApiError {
    name: ErrorName::Unknown,
    status: status.as_u16(),
    message,
    errors: Vec::new(),
    request_id: None,
  }
}

#[cfg(test)]
mod tests {
  use super::build_url;

  #[test]
  fn encodes_each_path_segment() {
    assert_eq!(
      build_url("http://x", &["v1", "contacts", "ada+news@acme.dev"]),
      "http://x/v1/contacts/ada%2Bnews@acme.dev"
    );

    assert_eq!(build_url("http://x", &["v1", "a/b c"]), "http://x/v1/a%2Fb%20c");
  }
}
