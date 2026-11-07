use serde::Deserialize;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
  #[error(transparent)]
  Api(#[from] ApiError),

  #[error(transparent)]
  Http(#[from] reqwest::Error),

  #[error("unexpected response body: {0}")]
  Decode(#[from] serde_json::Error),

  #[error("INBOXJOT_API_KEY is not set")]
  MissingApiKey,

  #[error("The image must be a PNG, JPEG or GIF file")]
  UnsupportedImage,

  #[error("Image uploads do not accept an idempotency key")]
  UnsupportedIdempotencyKey,
}

impl Error {
  pub fn name(&self) -> Option<ErrorName> {
    match self {
      Self::Api(error) => Some(error.name),
      _ => None,
    }
  }
}

#[derive(Clone, Debug, Deserialize, thiserror::Error)]
#[error("{message} ({status} {name:?})")]
pub struct ApiError {
  pub name: ErrorName,
  pub status: u16,
  pub message: String,

  #[serde(default)]
  pub errors: Vec<ErrorItem>,
  #[serde(default)]
  pub request_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ErrorItem {
  pub message: String,
  pub location: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorName {
  ValidationError,
  NotFound,
  Unauthorized,
  Forbidden,
  InternalServerError,
  BadRequest,
  UserNotVerified,
  RateLimitExceeded,
  Conflict,
  LimitExceeded,
  ReauthenticationRequired,
  InvalidIdempotencyKey,
  ConcurrentIdempotentRequests,
  InvalidIdempotentRequest,
  PayloadTooLarge,
  UnsupportedMediaType,
  ServiceUnavailable,
  UnknownError,
  #[serde(other)]
  Unknown,
}
