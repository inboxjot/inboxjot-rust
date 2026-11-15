use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Inboxjot;
use crate::error::Result;

#[derive(Clone, Debug)]
pub struct Events<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Events<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn send(&self, event: &Event) -> Result<TrackedEvent> {
    let path = ["v1", "events"];
    let idempotency_key = self.idempotency_key.as_deref();

    self
      .client
      .send(Method::POST, &path, Some(event), idempotency_key)
      .await
  }

  pub async fn batch(&self, events: &[Event]) -> Result<TrackedBatch> {
    let path = ["v1", "events", "batch"];
    let idempotency_key = self.idempotency_key.as_deref();

    self
      .client
      .send(Method::POST, &path, Some(events), idempotency_key)
      .await
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Event {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub email: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub occurred_at: Option<String>,
  #[serde(skip_serializing_if = "Map::is_empty")]
  pub data: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TrackedEvent {
  pub contact_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TrackedBatch {
  pub data: Vec<TrackedEvent>,
}
