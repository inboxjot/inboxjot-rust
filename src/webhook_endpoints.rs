use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::{PageParams, StatusOk};

#[derive(Clone, Debug)]
pub struct Webhooks<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Webhooks<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &PageParams) -> Result<WebhookEndpointPage> {
    let path = ["v1", "webhooks"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(
    &self,
    endpoint: &CreateWebhookEndpoint,
  ) -> Result<CreatedWebhookEndpoint> {
    let path = ["v1", "webhooks"];
    self
      .client
      .send(Method::POST, &path, Some(endpoint), self.key())
      .await
  }

  pub async fn get(&self, endpoint_id: &str) -> Result<WebhookEndpointDetail> {
    let path = ["v1", "webhooks", endpoint_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(
    &self,
    endpoint_id: &str,
    endpoint: &UpdateWebhookEndpoint,
  ) -> Result<WebhookEndpoint> {
    let path = ["v1", "webhooks", endpoint_id];
    self
      .client
      .send(Method::PATCH, &path, Some(endpoint), self.key())
      .await
  }

  pub async fn delete(&self, endpoint_id: &str) -> Result<StatusOk> {
    let path = ["v1", "webhooks", endpoint_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateWebhookEndpoint {
  pub name: String,
  pub url: String,
  pub events: Vec<WebhookEventType>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UpdateWebhookEndpoint {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub events: Option<Vec<WebhookEventType>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<EndpointStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WebhookEventType {
  #[serde(rename = "email.sent")]
  EmailSent,
  #[serde(rename = "email.delivered")]
  EmailDelivered,
  #[serde(rename = "email.opened")]
  EmailOpened,
  #[serde(rename = "email.clicked")]
  EmailClicked,
  #[serde(rename = "email.bounced")]
  EmailBounced,
  #[serde(rename = "email.complained")]
  EmailComplained,
  #[serde(rename = "email.delivery_delayed")]
  EmailDeliveryDelayed,
  #[serde(rename = "email.rejected")]
  EmailRejected,
  #[serde(rename = "email.failed")]
  EmailFailed,
  #[serde(rename = "email.unsubscribed")]
  EmailUnsubscribed,
  #[serde(rename = "contact.created")]
  ContactCreated,
  #[serde(rename = "contact.updated")]
  ContactUpdated,
  #[serde(rename = "contact.deleted")]
  ContactDeleted,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EndpointStatus {
  Enabled,
  Disabled,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct WebhookEndpoint {
  pub id: String,
  pub name: String,
  pub url: String,
  pub events: Vec<WebhookEventType>,
  pub status: EndpointStatus,
  pub disabled_reason: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CreatedWebhookEndpoint {
  pub id: String,
  pub name: String,
  pub url: String,
  pub events: Vec<WebhookEventType>,
  pub status: EndpointStatus,
  pub disabled_reason: Option<String>,
  pub created_at: String,
  pub updated_at: String,

  pub secret: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WebhookEndpointListItem {
  pub id: String,
  pub name: String,
  pub url: String,
  pub events: Vec<WebhookEventType>,
  pub status: EndpointStatus,
  pub disabled_reason: Option<String>,
  pub created_at: String,
  pub updated_at: String,

  pub success_rate: Option<f64>,
  pub last_delivery_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WebhookEndpointDetail {
  pub id: String,
  pub name: String,
  pub url: String,
  pub events: Vec<WebhookEventType>,
  pub status: EndpointStatus,
  pub disabled_reason: Option<String>,
  pub created_at: String,
  pub updated_at: String,

  pub deliveries_24h: i32,
  pub success_rate: Option<f64>,
  pub avg_duration_ms: Option<i32>,
  pub retrying: i32,
  pub recent_deliveries: Vec<WebhookDelivery>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct WebhookDelivery {
  pub delivery_id: String,
  pub event_id: String,
  pub event_type: String,
  pub endpoint_id: String,
  pub email_id: String,

  pub status: i32,
  pub attempt: i32,
  pub duration_ms: i32,

  pub created_at: String,
  pub last_attempt_at: String,
  pub next_attempt_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WebhookEndpointPage {
  pub data: Vec<WebhookEndpointListItem>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}
