use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::{PageParams, StatusOk};

#[derive(Clone, Debug)]
pub struct Domains<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Domains<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &PageParams) -> Result<DomainPage> {
    let path = ["v1", "domains"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(&self, domain: &CreateDomain) -> Result<Domain> {
    let path = ["v1", "domains"];
    self.client.send(Method::POST, &path, Some(domain), self.key()).await
  }

  pub async fn get(&self, domain_id: &str) -> Result<Domain> {
    let path = ["v1", "domains", domain_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn delete(&self, domain_id: &str) -> Result<StatusOk> {
    let path = ["v1", "domains", domain_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  pub async fn verify(&self, domain_id: &str) -> Result<Domain> {
    let path = ["v1", "domains", domain_id, "verify"];
    self.client.send(Method::POST, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateDomain {
  pub name: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub region: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub return_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Domain {
  pub id: String,
  pub name: String,
  pub mail_from_domain: String,
  pub status: DomainStatus,
  pub records: Vec<DomainRecord>,
  pub region: String,
  pub tls_policy: String,

  pub receiving_enabled: bool,
  pub receiving_target: String,
  pub receiving_status: DomainStatus,
  pub receiving_found_mx: Option<String>,
  pub receiving_checked_at: Option<String>,
  pub receiving_verified_at: Option<String>,

  pub open_tracking: bool,
  pub click_tracking: bool,
  pub tracking_domain: Option<String>,
  pub tracking_status: TrackingStatus,
  pub tracking_records: Vec<DomainRecord>,
  pub tracking_verified_at: Option<String>,

  pub mailing_address: Option<String>,
  pub verified_at: Option<String>,
  pub created_at: String,
  pub updated_at: String,

  pub sent_last_30_days: i32,
  pub bounced_last_30_days: i32,
  pub is_default: bool,
  pub receiving: DomainReceiving,
  pub warmup: DomainWarmup,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DomainRecord {
  pub kind: RecordKind,
  #[serde(rename = "type")]
  pub record_type: String,
  pub name: String,
  pub value: String,
  pub priority: Option<i32>,
  pub status: DomainStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DomainReceiving {
  pub host: String,
  pub mx: String,
  pub shared_address: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DomainWarmup {
  pub enabled: bool,
  pub started_on: Option<String>,
  pub day: Option<i64>,
  pub limit: Option<i64>,
  pub next_limit: Option<i64>,
  pub sent_today: i64,
  pub full_volume_on: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DomainListItem {
  pub id: String,
  pub name: String,
  pub status: DomainStatus,
  pub pending_record_kinds: Vec<RecordKind>,
  pub region: String,
  pub is_default: bool,
  pub open_tracking: bool,
  pub click_tracking: bool,
  pub sent_last_30_days: i32,
  pub verified_at: Option<String>,
  pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DomainPage {
  pub data: Vec<DomainListItem>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DomainStatus {
  NotStarted,
  Pending,
  Verified,
  Failed,
  TemporaryFailure,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RecordKind {
  Dkim,
  ReturnPath,
  Dmarc,
  TrackingCname,
  TrackingCaa,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TrackingStatus {
  NotStarted,
  Pending,
  Issuing,
  Verified,
  Failed,
  #[serde(other)]
  Unknown,
}
