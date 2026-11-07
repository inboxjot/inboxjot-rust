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
pub struct Templates<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Templates<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &ListTemplates) -> Result<TemplatePage> {
    let path = ["v1", "templates"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(&self, template: &CreateTemplate) -> Result<Template> {
    let path = ["v1", "templates"];
    self
      .client
      .send(Method::POST, &path, Some(template), self.key())
      .await
  }

  pub async fn get(&self, template_id: &str) -> Result<Template> {
    let path = ["v1", "templates", template_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(
    &self,
    template_id: &str,
    template: &UpdateTemplate,
  ) -> Result<Template> {
    let path = ["v1", "templates", template_id];
    self
      .client
      .send(Method::PATCH, &path, Some(template), self.key())
      .await
  }

  pub async fn delete(&self, template_id: &str) -> Result<StatusOk> {
    let path = ["v1", "templates", template_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  pub async fn publish(&self, template_id: &str) -> Result<Template> {
    let path = ["v1", "templates", template_id, "publish"];
    self.client.send(Method::POST, &path, NO_BODY, self.key()).await
  }

  pub async fn list_versions(
    &self,
    template_id: &str,
    params: &ListVersions,
  ) -> Result<DocumentVersionPage> {
    let path = ["v1", "templates", template_id, "versions"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create_version(
    &self,
    template_id: &str,
    version: &CreateVersion,
  ) -> Result<DocumentVersion> {
    let path = ["v1", "templates", template_id, "versions"];
    self.client.send(Method::POST, &path, Some(version), self.key()).await
  }

  pub async fn get_version(
    &self,
    template_id: &str,
    version_id: &str,
  ) -> Result<DocumentVersionDetail> {
    let path = ["v1", "templates", template_id, "versions", version_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update_version(
    &self,
    template_id: &str,
    version_id: &str,
    version: &UpdateVersion,
  ) -> Result<DocumentVersion> {
    let path = ["v1", "templates", template_id, "versions", version_id];
    self
      .client
      .send(Method::PATCH, &path, Some(version), self.key())
      .await
  }

  pub async fn restore_version(
    &self,
    template_id: &str,
    version_id: &str,
  ) -> Result<Template> {
    let path = ["v1", "templates", template_id, "versions", version_id, "restore"];
    self.client.send(Method::POST, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(clippy::option_option)]
pub struct CreateTemplate {
  pub name: String,

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
  pub starter_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(clippy::option_option)]
pub struct UpdateTemplate {
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
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListTemplates {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub sort: Option<TemplateSort>,
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
pub enum TemplateSort {
  Updated,
  Name,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Template {
  pub id: String,
  pub name: String,

  pub from: String,
  pub reply_to: Option<String>,
  pub subject: String,
  pub preheader: String,
  pub html: String,
  pub text: String,

  pub published_version_id: Option<String>,
  pub has_unpublished_changes: bool,

  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TemplateListItem {
  pub id: String,
  pub name: String,

  pub from: String,
  pub subject: String,
  pub preheader: String,

  pub stats: TemplateStats,
  pub used_in: UsedIn,

  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TemplateStats {
  pub sent: i64,
  pub opened: i64,
  pub clicked: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct UsedIn {
  pub count: i32,
  pub first: Option<UsedCampaign>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct UsedCampaign {
  pub id: String,
  pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TemplatePage {
  pub data: Vec<TemplateListItem>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}
