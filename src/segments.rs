use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::contacts::ContactStatus;
use crate::error::Result;
use crate::shared::{SortDirection, StatusOk};

#[derive(Clone, Debug)]
pub struct Segments<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Segments<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &ListSegments) -> Result<SegmentPage> {
    let path = ["v1", "segments"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(&self, segment: &CreateSegment) -> Result<Segment> {
    let path = ["v1", "segments"];
    self.client.send(Method::POST, &path, Some(segment), self.key()).await
  }

  pub async fn get(&self, segment_id: &str) -> Result<Segment> {
    let path = ["v1", "segments", segment_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(
    &self,
    segment_id: &str,
    segment: &UpdateSegment,
  ) -> Result<Segment> {
    let path = ["v1", "segments", segment_id];
    self
      .client
      .send(Method::PATCH, &path, Some(segment), self.key())
      .await
  }

  pub async fn delete(&self, segment_id: &str) -> Result<StatusOk> {
    let path = ["v1", "segments", segment_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  pub async fn list_members(
    &self,
    segment_id: &str,
    params: &ListSegmentMembers,
  ) -> Result<SegmentMemberPage> {
    let path = ["v1", "segments", segment_id, "members"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn add_members(
    &self,
    segment_id: &str,
    members: &SegmentMembers,
  ) -> Result<AddedMembers> {
    let path = ["v1", "segments", segment_id, "members"];
    self.client.send(Method::POST, &path, Some(members), self.key()).await
  }

  pub async fn remove_members(
    &self,
    segment_id: &str,
    members: &SegmentMembers,
  ) -> Result<RemovedMembers> {
    let path = ["v1", "segments", segment_id, "members"];
    self
      .client
      .send(Method::DELETE, &path, Some(members), self.key())
      .await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SegmentMembers {
  pub contact_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateSegment {
  pub name: String,
  #[serde(rename = "type")]
  pub kind: SegmentType,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub conditions: Option<SegmentConditions>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(clippy::option_option)]
pub struct UpdateSegment {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<Option<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub conditions: Option<SegmentConditions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentConditions {
  #[serde(rename = "match")]
  pub matching: SegmentMatch,
  pub rules: Vec<SegmentRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentRule {
  pub field: String,
  pub operator: SegmentOperator,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub value: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SegmentMatch {
  All,
  Any,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SegmentOperator {
  Is,
  IsNot,
  Contains,
  NotContains,
  StartsWith,
  EndsWith,
  IsSet,
  IsNotSet,
  Equals,
  NotEquals,
  GreaterThan,
  LessThan,
  Within,
  NotWithin,
  Before,
  After,
  In,
  NotIn,
  AtLeast,
  Never,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SegmentType {
  #[default]
  Static,
  Dynamic,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListSegments {
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub kind: Option<SegmentType>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub sort: Option<SegmentSort>,
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
pub enum SegmentSort {
  Created,
  Name,
  Contacts,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListSegmentMembers {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub before: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub per_page: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Segment {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "type")]
  pub kind: SegmentType,
  pub conditions: Option<SegmentConditions>,
  pub contact_count: Option<i32>,
  pub counted_at: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SegmentListItem {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  #[serde(rename = "type")]
  pub kind: SegmentType,
  pub contact_count: Option<i32>,
  pub counted_at: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SegmentPage {
  pub data: Vec<SegmentListItem>,
  pub total: u64,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SegmentMember {
  pub id: String,
  pub email: String,
  pub first_name: Option<String>,
  pub last_name: Option<String>,
  pub status: ContactStatus,
  pub joined_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SegmentMemberPage {
  pub data: Vec<SegmentMember>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AddedMembers {
  pub added: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RemovedMembers {
  pub removed: u64,
}
