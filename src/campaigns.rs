use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::{
  CreateVersion, DocumentVersion, DocumentVersionDetail, DocumentVersionPage,
  ListVersions, SortDirection, StatusOk, UpdateVersion,
};

#[derive(Clone, Debug)]
pub struct Campaigns<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Campaigns<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &ListCampaigns) -> Result<CampaignPage> {
    let path = ["v1", "campaigns"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(&self, campaign: &CampaignInput) -> Result<Campaign> {
    let path = ["v1", "campaigns"];
    self
      .client
      .send(Method::POST, &path, Some(campaign), self.key())
      .await
  }

  pub async fn get(&self, campaign_id: &str) -> Result<Campaign> {
    let path = ["v1", "campaigns", campaign_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(
    &self,
    campaign_id: &str,
    campaign: &CampaignInput,
  ) -> Result<Campaign> {
    let path = ["v1", "campaigns", campaign_id];
    self
      .client
      .send(Method::PATCH, &path, Some(campaign), self.key())
      .await
  }

  pub async fn delete(&self, campaign_id: &str) -> Result<StatusOk> {
    let path = ["v1", "campaigns", campaign_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  pub async fn send(&self, campaign_id: &str, send: &SendCampaign) -> Result<Campaign> {
    let path = ["v1", "campaigns", campaign_id, "send"];
    self.client.send(Method::POST, &path, Some(send), self.key()).await
  }

  pub async fn cancel(&self, campaign_id: &str) -> Result<Campaign> {
    let path = ["v1", "campaigns", campaign_id, "cancel"];
    self.client.send(Method::POST, &path, NO_BODY, self.key()).await
  }

  pub async fn send_test(
    &self,
    campaign_id: &str,
    test: &SendTestCampaign,
  ) -> Result<QueuedIds> {
    let path = ["v1", "campaigns", campaign_id, "test"];
    self.client.send(Method::POST, &path, Some(test), self.key()).await
  }

  pub async fn list_versions(
    &self,
    campaign_id: &str,
    params: &ListVersions,
  ) -> Result<DocumentVersionPage> {
    let path = ["v1", "campaigns", campaign_id, "versions"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create_version(
    &self,
    campaign_id: &str,
    version: &CreateVersion,
  ) -> Result<DocumentVersion> {
    let path = ["v1", "campaigns", campaign_id, "versions"];
    self.client.send(Method::POST, &path, Some(version), self.key()).await
  }

  pub async fn get_version(
    &self,
    campaign_id: &str,
    version_id: &str,
  ) -> Result<DocumentVersionDetail> {
    let path = ["v1", "campaigns", campaign_id, "versions", version_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update_version(
    &self,
    campaign_id: &str,
    version_id: &str,
    version: &UpdateVersion,
  ) -> Result<DocumentVersion> {
    let path = ["v1", "campaigns", campaign_id, "versions", version_id];
    self
      .client
      .send(Method::PATCH, &path, Some(version), self.key())
      .await
  }

  pub async fn restore_version(
    &self,
    campaign_id: &str,
    version_id: &str,
  ) -> Result<Campaign> {
    let path = ["v1", "campaigns", campaign_id, "versions", version_id, "restore"];
    self.client.send(Method::POST, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SendCampaign {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scheduled_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SendTestCampaign {
  pub to: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(clippy::option_option)]
pub struct CampaignInput {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub from: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reply_to: Option<Option<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub subject: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preheader: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub html: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<Map<String, Value>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub template_id: Option<Option<String>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub include_segment_ids: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclude_segment_ids: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub topic_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListCampaigns {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub sort: Option<CampaignSort>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub direction: Option<SortDirection>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub before: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub per_page: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignSort {
  Sent,
  Name,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Named {
  pub id: String,
  pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Campaign {
  pub id: String,
  pub name: String,
  pub status: CampaignStatus,

  pub from: String,
  pub reply_to: Option<String>,
  pub subject: String,
  pub preheader: String,
  pub html: Option<String>,
  pub text: Option<String>,

  pub template_id: Option<String>,
  pub template: Option<Named>,
  pub include_segment_ids: Vec<String>,
  pub include_segments: Vec<Named>,
  pub exclude_segment_ids: Vec<String>,
  pub exclude_segments: Vec<Named>,
  pub topic_id: String,
  pub topic: Option<Named>,

  pub audience_count: Option<i32>,
  pub queued_count: i32,
  pub skipped_count: i32,
  pub paused_reason: Option<CampaignPauseReason>,
  pub failure_reason: Option<String>,

  pub scheduled_at: Option<String>,
  pub started_at: Option<String>,
  pub completed_at: Option<String>,
  pub warmup_next_limit: Option<i64>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CampaignListItem {
  pub id: String,
  pub name: String,
  pub status: CampaignStatus,
  pub subject: String,

  pub include_segments: Vec<Named>,
  pub exclude_segments: Vec<Named>,
  pub topic: Option<Named>,

  pub audience_count: Option<i32>,
  pub queued_count: i32,
  pub skipped_count: i32,
  pub paused_reason: Option<CampaignPauseReason>,

  pub scheduled_at: Option<String>,
  pub started_at: Option<String>,
  pub completed_at: Option<String>,
  pub created_at: String,
  pub updated_at: String,

  pub stats: Option<CampaignSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CampaignSummary {
  pub delivered: i32,
  pub opened: i32,
  pub clicked: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CampaignPage {
  pub data: Vec<CampaignListItem>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CampaignStatus {
  Draft,
  Scheduled,
  Sending,
  Paused,
  Failed,
  Sent,
  Canceled,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CampaignPauseReason {
  Manual,
  DailyLimit,
  Warmup,
  SendingPaused,
  DomainUnverified,
  NoPlan,
  PlanLimit,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct QueuedIds {
  pub ids: Vec<String>,
}
