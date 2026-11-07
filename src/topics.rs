use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::{PageParams, StatusOk};

#[derive(Clone, Debug)]
pub struct Topics<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Topics<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &PageParams) -> Result<TopicPage> {
    let path = ["v1", "topics"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(&self, topic: &CreateTopic) -> Result<Topic> {
    let path = ["v1", "topics"];
    self.client.send(Method::POST, &path, Some(topic), self.key()).await
  }

  pub async fn get(&self, topic_id: &str) -> Result<Topic> {
    let path = ["v1", "topics", topic_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(&self, topic_id: &str, topic: &UpdateTopic) -> Result<Topic> {
    let path = ["v1", "topics", topic_id];
    self.client.send(Method::PATCH, &path, Some(topic), self.key()).await
  }

  pub async fn delete(&self, topic_id: &str) -> Result<StatusOk> {
    let path = ["v1", "topics", topic_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateTopic {
  pub name: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default_subscribed: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub visibility: Option<TopicVisibility>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub double_opt_in: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub form_enabled: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sender_name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sender_email: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub redirect_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(clippy::option_option)]
pub struct UpdateTopic {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<Option<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default_subscribed: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub visibility: Option<TopicVisibility>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub double_opt_in: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub form_enabled: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sender_name: Option<Option<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sender_email: Option<Option<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub redirect_url: Option<Option<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TopicVisibility {
  Public,
  Private,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Topic {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  pub default_subscribed: bool,
  pub visibility: TopicVisibility,

  pub double_opt_in: bool,
  pub form_enabled: bool,
  pub sender_name: Option<String>,
  pub sender_email: Option<String>,
  pub redirect_url: Option<String>,

  pub is_default: bool,
  pub counts: TopicCounts,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TopicCounts {
  pub subscribed: i32,
  pub pending: i32,
  pub unsubscribed: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TopicPage {
  pub data: Vec<Topic>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}
