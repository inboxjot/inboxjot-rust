use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::Inboxjot;
use crate::client::NO_BODY;
use crate::error::{Error, Result};
use crate::shared::{SortDirection, StatusOk};

#[derive(Clone, Debug)]
pub struct Images<'a> {
  client: &'a Inboxjot,
  idempotency_key: Option<String>,
}

impl<'a> Images<'a> {
  pub(crate) fn new(client: &'a Inboxjot) -> Self {
    Self { client, idempotency_key: None }
  }

  pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
    self.idempotency_key = Some(key.into());
    self
  }

  pub async fn list(&self, params: &ListImages) -> Result<ImagePage> {
    let path = ["v1", "images"];
    self
      .client
      .send_with_query(Method::GET, &path, Some(params), NO_BODY, None)
      .await
  }

  pub async fn upload(&self, image: &UploadImage) -> Result<Image> {
    if self.idempotency_key.is_some() {
      return Err(Error::UnsupportedIdempotencyKey);
    }
    let content_type = get_image_content_type(&image.file)?;

    let body = UploadUrlBody { content_type, size: image.file.len() };
    let target: ImageUploadTarget = self
      .client
      .send(Method::POST, &["v1", "images", "presign"], Some(&body), None)
      .await?;

    self
      .client
      .put_upload(&target.upload_url, image.file.clone(), content_type)
      .await?;

    let body = CreateImageBody {
      key: &target.key,
      file_name: image.file_name.as_deref(),
    };
    self
      .client
      .send(Method::POST, &["v1", "images"], Some(&body), None)
      .await
  }

  pub async fn get(&self, image_id: &str) -> Result<Image> {
    let path = ["v1", "images", image_id];
    self.client.send(Method::GET, &path, NO_BODY, None).await
  }

  pub async fn delete(&self, image_id: &str) -> Result<StatusOk> {
    let path = ["v1", "images", image_id];
    self.client.send(Method::DELETE, &path, NO_BODY, self.key()).await
  }

  fn key(&self) -> Option<&str> {
    self.idempotency_key.as_deref()
  }
}

fn get_image_content_type(bytes: &[u8]) -> Result<&'static str> {
  if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
    Ok("image/png")
  } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
    Ok("image/jpeg")
  } else if bytes.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
    Ok("image/gif")
  } else {
    Err(Error::UnsupportedImage)
  }
}

#[derive(Serialize)]
struct UploadUrlBody {
  content_type: &'static str,
  size: usize,
}

#[derive(Deserialize)]
struct ImageUploadTarget {
  upload_url: String,
  key: String,
}

#[derive(Serialize)]
struct CreateImageBody<'a> {
  key: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  file_name: Option<&'a str>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UploadImage {
  pub file: Vec<u8>,
  pub file_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListImages {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub query: Option<String>,
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub kind: Option<ImageType>,
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
pub enum ImageType {
  Png,
  Jpeg,
  Gif,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Image {
  pub id: String,
  pub file_name: String,
  pub url: Option<String>,
  pub content_type: String,
  pub size_bytes: i32,
  pub width: i32,
  pub height: i32,
  pub creator_name: Option<String>,
  pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ImagePage {
  pub data: Vec<Image>,
  pub total: u64,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}
