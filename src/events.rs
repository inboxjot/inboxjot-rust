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
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Event {
  pub name: String,
  pub email: String,
  #[serde(skip_serializing_if = "Map::is_empty")]
  pub data: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TrackedEvent {
  pub contact_id: String,
}
