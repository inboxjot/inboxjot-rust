use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct StatusOk {
  pub status: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
  Asc,
  Desc,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PageParams {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub before: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub per_page: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ListVersions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub filter: Option<VersionFilter>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub before: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub per_page: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionFilter {
  All,
  Saved,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateVersion {
  pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UpdateVersion {
  pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DocumentVersion {
  pub id: String,
  pub reason: DocumentVersionReason,
  pub name: Option<String>,
  pub editors: Vec<DocumentVersionEditor>,
  pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DocumentVersionDetail {
  pub id: String,
  pub reason: DocumentVersionReason,
  pub name: Option<String>,
  pub editors: Vec<DocumentVersionEditor>,

  pub from: String,
  pub reply_to: Option<String>,
  pub subject: String,
  pub preheader: String,
  pub html: String,
  pub text: String,
  pub content: Value,

  pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DocumentVersionEditor {
  pub kind: DocumentVersionEditorKind,
  pub id: String,
  pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DocumentVersionEditorKind {
  User,
  Agent,
  Api,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DocumentVersionReason {
  Checkpoint,
  Manual,
  BeforeAgentEdit,
  BeforeRestore,
  Restored,
  Published,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DocumentVersionPage {
  pub data: Vec<DocumentVersion>,
  pub total: u64,
  pub next_cursor: Option<String>,
  pub prev_cursor: Option<String>,
}
