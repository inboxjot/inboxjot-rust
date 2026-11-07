use std::collections::BTreeMap;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::SortDirection;

#[derive(Clone, Debug)]
pub struct Emails<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Emails<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn send(&self, email: &SendEmail) -> Result<QueuedEmail> {
    let path = ["v1", "emails"];
    self.client.send(Method::POST, &path, Some(email), self.key()).await
  }

  pub async fn batch(&self, emails: &[BatchEmail]) -> Result<QueuedBatch> {
    let path = ["v1", "emails", "batch"];
    self.client.send(Method::POST, &path, Some(emails), self.key()).await
  }

  pub async fn list(&self, params: &ListEmails) -> Result<EmailPage> {
    let path = ["v1", "emails"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn get(&self, email_id: &str) -> Result<Email> {
    let path = ["v1", "emails", email_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(
    &self,
    email_id: &str,
    email: &UpdateEmail,
  ) -> Result<RescheduledEmail> {
    let path = ["v1", "emails", email_id];
    self.client.send(Method::PATCH, &path, Some(email), self.key()).await
  }

  pub async fn cancel(&self, email_id: &str) -> Result<CanceledEmail> {
    let path = ["v1", "emails", email_id, "cancel"];
    self.client.send(Method::POST, &path, NO_BODY, self.key()).await
  }

  pub async fn list_attachments(&self, email_id: &str) -> Result<AttachmentList> {
    let path = ["v1", "emails", email_id, "attachments"];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn get_attachment(
    &self,
    email_id: &str,
    attachment_id: &str,
  ) -> Result<Attachment> {
    let path = ["v1", "emails", email_id, "attachments", attachment_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UpdateEmail {
  pub scheduled_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SendEmail {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub from: Option<String>,
  pub to: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub cc: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub bcc: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub reply_to: Vec<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub subject: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub template: Option<TemplateRef>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub html: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,

  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  pub headers: BTreeMap<String, String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tags: Vec<Tag>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scheduled_at: Option<String>,

  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub attachments: Vec<AttachmentInput>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct BatchEmail {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub from: Option<String>,
  pub to: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub cc: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub bcc: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub reply_to: Vec<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub subject: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub template: Option<TemplateRef>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub html: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,

  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  pub headers: BTreeMap<String, String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TemplateRef {
  pub id: String,
  #[serde(skip_serializing_if = "Map::is_empty")]
  pub variables: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AttachmentInput {
  pub filename: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct QueuedEmail {
  pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct QueuedBatch {
  pub data: Vec<QueuedEmail>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Email {
  pub id: String,
  pub from: String,
  pub to: Vec<String>,
  pub cc: Vec<String>,
  pub bcc: Vec<String>,

  pub subject: String,
  pub tags: Vec<Tag>,
  pub status: EmailStatus,
  pub source: EmailSource,

  pub scheduled_at: Option<String>,
  pub sent_at: Option<String>,
  pub created_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EmailStatus {
  Scheduled,
  Queued,
  Sending,
  Sent,
  SoftBounced,
  Delivered,
  Opened,
  Clicked,
  Bounced,
  Rejected,
  Failed,
  Canceled,
  Complained,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EmailSource {
  Api,
  Smtp,
  Dashboard,
  Mcp,
  Campaign,
  Automation,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListEmails {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status_op: Option<EmailStatusOp>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub source: Option<EmailSource>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub sent_after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sent_before: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub sort: Option<EmailSort>,
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
pub enum EmailStatusOp {
  Is,
  Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailSort {
  Sent,
  To,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailPage {
  pub data: Vec<Email>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RescheduledEmail {
  pub id: String,
  pub scheduled_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CanceledEmail {
  pub id: String,
  pub status: EmailStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AttachmentList {
  pub data: Vec<Attachment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Attachment {
  pub id: String,
  pub filename: String,
  pub content_type: String,
  pub size: i32,
  pub content_id: Option<String>,
  pub content_disposition: String,

  pub deleted_at: Option<String>,
  pub download_url: Option<String>,
  pub expires_at: Option<String>,
}
