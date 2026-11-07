use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::Sha256;

use crate::contacts::ContactStatus;
use crate::emails::Tag;

const TOLERANCE_SECONDS: u64 = 5 * 60;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
  #[error("the signing secret is not valid")]
  InvalidSecret,

  #[error("the {0} header is missing")]
  MissingHeader(&'static str),

  #[error("the webhook-timestamp header is not valid")]
  InvalidTimestamp,

  #[error("the webhook timestamp is too old or too new")]
  ExpiredTimestamp,

  #[error("no webhook signature matches")]
  InvalidSignature,

  #[error("the webhook body is not a valid event: {0}")]
  InvalidPayload(#[from] serde_json::Error),
}

pub fn verify(
  secret: &str,
  headers: &HeaderMap,
  body: &[u8],
) -> Result<WebhookEvent, Error> {
  let seconds_since_epoch = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |elapsed| elapsed.as_secs());
  let now = i64::try_from(seconds_since_epoch).unwrap_or(i64::MAX);

  check_signature(secret, headers, body, now)?;

  Ok(serde_json::from_slice(body)?)
}

fn check_signature(
  secret: &str,
  headers: &HeaderMap,
  body: &[u8],
  now: i64,
) -> Result<(), Error> {
  let encoded_key = secret.strip_prefix("whsec_").unwrap_or(secret);
  let key = STANDARD.decode(encoded_key).map_err(|_| Error::InvalidSecret)?;

  let id = read_header(headers, "webhook-id")?;
  let timestamp = read_header(headers, "webhook-timestamp")?;
  let signatures = read_header(headers, "webhook-signature")?;

  let sent_at: i64 = timestamp.parse().map_err(|_| Error::InvalidTimestamp)?;
  if now.abs_diff(sent_at) > TOLERANCE_SECONDS {
    return Err(Error::ExpiredTimestamp);
  }

  let mut mac = Hmac::<Sha256>::new_from_slice(&key).map_err(|_| Error::InvalidSecret)?;
  mac.update(format!("{id}.{timestamp}.").as_bytes());
  mac.update(body);

  let is_match = signatures
    .split(' ')
    .filter_map(|entry| entry.strip_prefix("v1,"))
    .filter_map(|signature| STANDARD.decode(signature).ok())
    .any(|signature| mac.clone().verify_slice(&signature).is_ok());

  if is_match { Ok(()) } else { Err(Error::InvalidSignature) }
}

fn read_header<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, Error> {
  headers
    .get(name)
    .and_then(|value| value.to_str().ok())
    .ok_or(Error::MissingHeader(name))
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "RawEvent")]
pub struct WebhookEvent {
  pub id: String,
  pub created_at: String,
  pub data: WebhookEventData,
}

#[derive(Deserialize)]
struct RawEvent {
  id: String,
  created_at: String,
  #[serde(rename = "type")]
  kind: String,
  data: Value,
}

impl TryFrom<RawEvent> for WebhookEvent {
  type Error = serde_json::Error;

  fn try_from(raw: RawEvent) -> Result<Self, Self::Error> {
    let data = if EVENT_TYPES.contains(&raw.kind.as_str()) {
      let tagged = serde_json::json!({ "type": raw.kind, "data": raw.data });
      serde_json::from_value(tagged)?
    } else {
      WebhookEventData::Unknown { kind: raw.kind, data: raw.data }
    };

    Ok(Self { id: raw.id, created_at: raw.created_at, data })
  }
}

const EVENT_TYPES: [&str; 13] = [
  "email.sent",
  "email.delivered",
  "email.opened",
  "email.clicked",
  "email.bounced",
  "email.complained",
  "email.delivery_delayed",
  "email.rejected",
  "email.failed",
  "email.unsubscribed",
  "contact.created",
  "contact.updated",
  "contact.deleted",
];

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", content = "data")]
#[non_exhaustive]
pub enum WebhookEventData {
  #[serde(rename = "email.sent")]
  EmailSent(EmailEventData),
  #[serde(rename = "email.delivered")]
  EmailDelivered(EmailEventData),
  #[serde(rename = "email.opened")]
  EmailOpened(EmailOpenedData),
  #[serde(rename = "email.clicked")]
  EmailClicked(EmailClickedData),
  #[serde(rename = "email.bounced")]
  EmailBounced(EmailBouncedData),
  #[serde(rename = "email.complained")]
  EmailComplained(EmailComplainedData),
  #[serde(rename = "email.delivery_delayed")]
  EmailDeliveryDelayed(EmailEventData),
  #[serde(rename = "email.rejected")]
  EmailRejected(EmailFailedData),
  #[serde(rename = "email.failed")]
  EmailFailed(EmailFailedData),
  #[serde(rename = "email.unsubscribed")]
  EmailUnsubscribed(EmailEventData),

  #[serde(rename = "contact.created")]
  ContactCreated(ContactEventData),
  #[serde(rename = "contact.updated")]
  ContactUpdated(ContactEventData),
  #[serde(rename = "contact.deleted")]
  ContactDeleted(ContactEventData),

  #[serde(skip)]
  Unknown { kind: String, data: Value },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailEventData {
  pub email_id: String,
  pub from: String,
  pub to: Vec<String>,
  pub subject: String,
  #[serde(default)]
  pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailOpenedData {
  #[serde(flatten)]
  pub email: EmailEventData,
  pub actor: EmailActor,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailClickedData {
  #[serde(flatten)]
  pub email: EmailEventData,
  pub actor: EmailActor,
  pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailBouncedData {
  #[serde(flatten)]
  pub email: EmailEventData,
  pub bounce: Bounce,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailComplainedData {
  #[serde(flatten)]
  pub email: EmailEventData,
  pub complaint: Complaint,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmailFailedData {
  #[serde(flatten)]
  pub email: EmailEventData,
  pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EmailActor {
  Human,
  AppleMpp,
  Scanner,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Bounce {
  #[serde(rename = "type")]
  pub kind: BounceType,
  pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BounceType {
  Permanent,
  Transient,
  Undetermined,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Complaint {
  #[serde(rename = "type")]
  pub kind: ComplaintType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ComplaintType {
  Abuse,
  #[serde(other)]
  Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ContactEventData {
  pub contact_id: String,
  pub email: String,
  pub first_name: Option<String>,
  pub last_name: Option<String>,

  pub status: ContactStatus,
  pub language: String,
  pub properties: Map<String, Value>,

  pub created_at: String,
  pub updated_at: String,
}

#[cfg(test)]
mod tests {
  use base64::Engine;
  use base64::engine::general_purpose::STANDARD;
  use hmac::{Hmac, KeyInit, Mac};
  use reqwest::header::HeaderMap;
  use serde_json::json;
  use sha2::Sha256;

  use super::{BounceType, EmailActor, Error, WebhookEventData, check_signature, verify};

  const SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
  const ID: &str = "msg_p5jXN8AQM9LWM0D4loKWxJek";
  const TIMESTAMP: i64 = 1_614_265_330;
  const BODY: &str = r#"{"test": 2432232314}"#;
  const SIGNATURE: &str = "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=";

  fn build_headers(id: &str, timestamp: i64, signature: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("webhook-id", id.parse().unwrap());
    headers.insert("webhook-timestamp", timestamp.to_string().parse().unwrap());
    headers.insert("webhook-signature", signature.parse().unwrap());

    headers
  }

  fn sign(secret: &str, id: &str, timestamp: i64, body: &str) -> String {
    let key = STANDARD.decode(secret.trim_start_matches("whsec_")).unwrap();

    let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
    mac.update(format!("{id}.{timestamp}.{body}").as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());

    format!("v1,{signature}")
  }

  fn now() -> i64 {
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    i64::try_from(elapsed.unwrap().as_secs()).unwrap()
  }

  fn build_signed_headers(id: &str, body: &str) -> HeaderMap {
    let timestamp = now();
    let signature = sign(SECRET, id, timestamp, body);

    build_headers(id, timestamp, &signature)
  }

  #[test]
  fn accepts_the_standard_webhooks_vector() {
    let headers = build_headers(ID, TIMESTAMP, SIGNATURE);
    let result = check_signature(SECRET, &headers, BODY.as_bytes(), TIMESTAMP);

    assert!(result.is_ok());
  }

  #[test]
  fn signs_like_the_api() {
    assert_eq!(sign(SECRET, ID, TIMESTAMP, BODY), SIGNATURE);
  }

  #[test]
  fn accepts_any_matching_signature_in_the_list() {
    let signatures = format!("v1,bm90IGl0 v2,abc {SIGNATURE}");
    let headers = build_headers(ID, TIMESTAMP, &signatures);
    let result = check_signature(SECRET, &headers, BODY.as_bytes(), TIMESTAMP);

    assert!(result.is_ok());
  }

  #[test]
  fn accepts_a_secret_made_before_the_switch() {
    let secret = "whsec_3kTmPqZ8vXw2RbN7cYhJ4dLsF9gA6eUu";
    let signature = sign(secret, "evt_1", 1, "{}");
    let headers = build_headers("evt_1", 1, &signature);

    assert!(check_signature(secret, &headers, b"{}", 1).is_ok());
  }

  #[test]
  fn rejects_a_changed_body() {
    let headers = build_headers(ID, TIMESTAMP, SIGNATURE);
    let result = check_signature(SECRET, &headers, b"{\"test\": 1}", TIMESTAMP);

    assert!(matches!(result, Err(Error::InvalidSignature)));
  }

  #[test]
  fn rejects_an_old_timestamp() {
    let headers = build_headers(ID, TIMESTAMP, SIGNATURE);
    let result = check_signature(SECRET, &headers, BODY.as_bytes(), TIMESTAMP + 301);

    assert!(matches!(result, Err(Error::ExpiredTimestamp)));
  }

  #[test]
  fn rejects_a_timestamp_at_the_integer_limit() {
    let headers = build_headers(ID, i64::MIN, SIGNATURE);
    let result = check_signature(SECRET, &headers, BODY.as_bytes(), TIMESTAMP);

    assert!(matches!(result, Err(Error::ExpiredTimestamp)));
  }

  #[test]
  fn rejects_a_missing_header() {
    let mut headers = build_headers(ID, TIMESTAMP, SIGNATURE);
    headers.remove("webhook-signature");

    let result = check_signature(SECRET, &headers, BODY.as_bytes(), TIMESTAMP);

    assert!(matches!(result, Err(Error::MissingHeader("webhook-signature"))));
  }

  #[test]
  fn parses_a_signed_email_event() {
    let body = json!({
        "id": "evt_1",
        "type": "email.bounced",
        "created_at": "2026-10-10T09:00:00.000Z",
        "data": {
            "email_id": "eml_1",
            "to": ["ada@acme.dev"],
            "from": "hi@acme.dev",
            "subject": "Hi",
            "tags": [{ "name": "kind", "value": "receipt" }],
            "bounce": { "type": "permanent", "message": "No such user" }
        }
    })
    .to_string();
    let headers = build_signed_headers("evt_1", &body);

    let event = verify(SECRET, &headers, body.as_bytes()).unwrap();
    assert_eq!(event.id, "evt_1");

    let WebhookEventData::EmailBounced(data) = event.data else {
      panic!("expected email.bounced, got {:?}", event.data);
    };
    assert_eq!(data.bounce.kind, BounceType::Permanent);
    assert_eq!(data.email.tags[0].value, "receipt");
  }

  #[test]
  fn parses_an_opened_event_with_a_new_actor() {
    let body = json!({
        "id": "evt_4",
        "type": "email.opened",
        "created_at": "2026-10-10T09:00:00.000Z",
        "data": {
            "email_id": "eml_1",
            "to": ["ada@acme.dev"],
            "from": "hi@acme.dev",
            "subject": "Hi",
            "actor": "robot"
        }
    })
    .to_string();
    let headers = build_signed_headers("evt_4", &body);

    let event = verify(SECRET, &headers, body.as_bytes()).unwrap();

    let WebhookEventData::EmailOpened(data) = event.data else {
      panic!("expected email.opened, got {:?}", event.data);
    };
    assert_eq!(data.actor, EmailActor::Unknown);
    assert!(data.email.tags.is_empty());
  }

  #[test]
  fn parses_a_clicked_event_without_a_link() {
    let body = json!({
        "id": "evt_5",
        "type": "email.clicked",
        "created_at": "2026-10-10T09:00:00.000Z",
        "data": {
            "email_id": "eml_1",
            "to": ["ada@acme.dev"],
            "from": "hi@acme.dev",
            "subject": "Hi",
            "actor": "unknown"
        }
    })
    .to_string();
    let headers = build_signed_headers("evt_5", &body);

    let event = verify(SECRET, &headers, body.as_bytes()).unwrap();

    let WebhookEventData::EmailClicked(data) = event.data else {
      panic!("expected email.clicked, got {:?}", event.data);
    };
    assert_eq!(data.actor, EmailActor::Unknown);
    assert_eq!(data.link, None);
  }

  #[test]
  fn parses_a_contact_event() {
    let body = json!({
        "id": "evt_2",
        "type": "contact.created",
        "created_at": "2026-10-10T09:00:00.000Z",
        "data": {
            "contact_id": "con_1",
            "email": "ada@acme.dev",
            "first_name": "Ada",
            "last_name": null,
            "status": "subscribed",
            "language": "en",
            "properties": { "plan": "pro" },
            "created_at": "2026-10-10T09:00:00.000Z",
            "updated_at": "2026-10-10T09:00:00.000Z"
        }
    })
    .to_string();
    let headers = build_signed_headers("evt_2", &body);

    let event = verify(SECRET, &headers, body.as_bytes()).unwrap();

    let WebhookEventData::ContactCreated(data) = event.data else {
      panic!("expected contact.created, got {:?}", event.data);
    };
    assert_eq!(data.first_name.as_deref(), Some("Ada"));
    assert_eq!(data.status, crate::ContactStatus::Subscribed);
  }

  #[test]
  fn parses_an_unknown_event_type() {
    let body = json!({
        "id": "evt_3",
        "type": "domain.verified",
        "created_at": "2026-10-10T09:00:00.000Z",
        "data": { "domain_id": "dom_1" }
    })
    .to_string();
    let headers = build_signed_headers("evt_3", &body);

    let event = verify(SECRET, &headers, body.as_bytes()).unwrap();

    let WebhookEventData::Unknown { kind, .. } = event.data else {
      panic!("expected an unknown event, got {:?}", event.data);
    };
    assert_eq!(kind, "domain.verified");
  }
}
