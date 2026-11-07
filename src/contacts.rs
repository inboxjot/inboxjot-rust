use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::{SortDirection, StatusOk};

#[derive(Clone, Debug)]
pub struct Contacts<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Contacts<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn create(&self, contact: &CreateContact) -> Result<SavedContact> {
    let path = ["v1", "contacts"];
    self.client.send(Method::POST, &path, Some(contact), self.key()).await
  }

  pub async fn list(&self, params: &ListContacts) -> Result<ContactListPage> {
    let path = ["v1", "contacts"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn get(&self, contact_id_or_email: &str) -> Result<Contact> {
    let path = ["v1", "contacts", contact_id_or_email];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(
    &self,
    contact_id_or_email: &str,
    contact: &UpdateContact,
  ) -> Result<Contact> {
    let path = ["v1", "contacts", contact_id_or_email];
    self
      .client
      .send(Method::PATCH, &path, Some(contact), self.key())
      .await
  }

  pub async fn delete(&self, contact_id_or_email: &str) -> Result<StatusOk> {
    let path = ["v1", "contacts", contact_id_or_email];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  pub async fn erase(&self, erasure: &EraseContact) -> Result<QueuedErasure> {
    let path = ["v1", "contacts", "erase"];
    self.client.send(Method::POST, &path, Some(erasure), self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateContact {
  pub email: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub first_name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_name: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub language: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub notes: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub properties: Option<Map<String, Value>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub subscribed: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub segment_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(clippy::option_option)]
pub struct UpdateContact {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub first_name: Option<Option<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_name: Option<Option<String>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub language: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub notes: Option<Option<String>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub properties: Option<Map<String, Value>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub subscribed: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub segment_ids: Option<Vec<String>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub topics: Option<Vec<TopicChoice>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TopicChoice {
  pub id: String,
  pub status: TopicStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicStatus {
  Subscribed,
  Unsubscribed,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SavedContact {
  pub id: String,
  pub email: String,
  pub first_name: Option<String>,
  pub last_name: Option<String>,

  pub status: ContactStatus,
  pub language: String,
  pub properties: Map<String, Value>,

  pub segments: Vec<SegmentRef>,
  pub topics: Vec<ContactTopic>,

  pub created_at: String,
  pub updated_at: String,
  pub created: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Contact {
  pub id: String,
  pub email: String,
  pub first_name: Option<String>,
  pub last_name: Option<String>,

  pub status: ContactStatus,
  pub language: String,
  pub properties: Map<String, Value>,

  pub segments: Vec<SegmentRef>,
  pub topics: Vec<ContactTopic>,

  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SegmentRef {
  pub id: String,
  pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ContactTopic {
  pub id: String,
  pub name: String,
  pub status: Option<ContactStatus>,
  pub subscribed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContactStatus {
  Subscribed,
  Pending,
  NotSubscribed,
  Unsubscribed,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct EraseContact {
  pub email: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ErasureStatus {
  Queued,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct QueuedErasure {
  pub status: ErasureStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListContacts {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sort: Option<ContactSort>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub direction: Option<SortDirection>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<ContactStatus>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub segment_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub before: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub per_page: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactSort {
  Added,
  Contact,
  LastActivity,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ContactListPage {
  pub data: Vec<ContactListItem>,
  pub total: u64,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ContactListItem {
  pub id: String,
  pub email: String,
  pub first_name: Option<String>,
  pub last_name: Option<String>,

  pub status: ContactStatus,
  pub is_suppressed: bool,
  pub properties: Map<String, Value>,

  pub segments: Vec<SegmentRef>,
  pub last_activity: Option<LastActivity>,
  pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct LastActivity {
  #[serde(rename = "type")]
  pub kind: String,
  pub at: String,
}
