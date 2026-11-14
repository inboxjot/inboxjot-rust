use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::Result;
use crate::shared::{PageParams, StatusOk};

#[derive(Clone, Debug)]
pub struct Brands<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Brands<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &PageParams) -> Result<BrandPage> {
    let path = ["v1", "brands"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn create(&self, brand: &BrandInput) -> Result<Brand> {
    let path = ["v1", "brands"];
    self.client.send(Method::POST, &path, Some(brand), self.key()).await
  }

  pub async fn get(&self, brand_id: &str) -> Result<Brand> {
    let path = ["v1", "brands", brand_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn update(&self, brand_id: &str, brand: &BrandInput) -> Result<Brand> {
    let path = ["v1", "brands", brand_id];
    self.client.send(Method::PATCH, &path, Some(brand), self.key()).await
  }

  pub async fn delete(&self, brand_id: &str) -> Result<StatusOk> {
    let path = ["v1", "brands", brand_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BrandInput {
  pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Brand {
  pub id: String,
  pub name: String,
  pub slug: String,
  pub logo_url: Option<String>,
  pub is_default: bool,
  pub default_topic_id: Option<String>,
  pub topic_count: i64,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct BrandPage {
  pub data: Vec<Brand>,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}
