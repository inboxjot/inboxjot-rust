use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use inboxjot::{
  BatchEmail, CreateVersion, EraseContact, ErasureStatus, Error, ErrorName, Inboxjot,
  SegmentMembers, SendCampaign, SendEmail, SendTestCampaign, UpdateContact, UpdateEmail,
  UpdateVersion, UploadImage,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct Reply {
  status: u16,
  headers: &'static str,
  body: String,
}

#[derive(Clone, Debug)]
struct Recorded {
  method: String,
  path: String,
  headers: Vec<(String, String)>,
  body: String,
  bytes: Vec<u8>,
}

impl Recorded {
  fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(key, _)| key == name)
      .map(|(_, value)| value.as_str())
  }

  fn json(&self) -> Value {
    serde_json::from_str(&self.body).unwrap()
  }
}

struct Mock {
  url: String,
  requests: Arc<Mutex<Vec<Recorded>>>,
}

impl Mock {
  fn client(&self) -> Inboxjot {
    Inboxjot::builder("ibx_test").base_url(&self.url).build()
  }

  fn requests(&self) -> Vec<Recorded> {
    self.requests.lock().unwrap().clone()
  }
}

fn reply(status: u16, body: impl ToString) -> Option<Reply> {
  reply_with(status, "", body)
}

fn reply_with(status: u16, headers: &'static str, body: impl ToString) -> Option<Reply> {
  let body = body.to_string();
  Some(Reply { status, headers, body })
}

async fn start_mock(replies: Vec<Option<Reply>>) -> Mock {
  start_mock_with(|_| replies).await
}

async fn start_mock_with(build: impl FnOnce(&str) -> Vec<Option<Reply>>) -> Mock {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let replies = build(&url);

  let requests = Arc::new(Mutex::new(Vec::new()));
  let queue = Arc::new(Mutex::new(VecDeque::from(replies)));
  let recorded = requests.clone();

  tokio::spawn(async move {
    loop {
      let (stream, _) = listener.accept().await.unwrap();
      let recorded = recorded.clone();
      let queue = queue.clone();

      tokio::spawn(async move {
        let (request, stream) = read_request(stream).await;
        recorded.lock().unwrap().push(request);

        let next = queue.lock().unwrap().pop_front().flatten();
        if let Some(reply) = next {
          write_reply(stream, reply).await;
        }
      });
    }
  });

  Mock { url, requests }
}

async fn read_request(mut stream: TcpStream) -> (Recorded, TcpStream) {
  let mut buffer = Vec::new();
  let mut chunk = [0_u8; 4096];

  let head_end = loop {
    let bytes_read = stream.read(&mut chunk).await.unwrap();
    buffer.extend_from_slice(&chunk[..bytes_read]);

    let separator = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    if let Some(position) = separator {
      break position + 4;
    }
  };

  let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
  let mut lines = head.lines();
  let mut request_line = lines.next().unwrap().split(' ');
  let method = request_line.next().unwrap().to_owned();
  let path = request_line.next().unwrap().to_owned();

  let headers: Vec<(String, String)> = lines
    .filter_map(|line| line.split_once(':'))
    .map(|(key, value)| (key.to_lowercase(), value.trim().to_owned()))
    .collect();
  let content_length = headers
    .iter()
    .find(|(key, _)| key == "content-length")
    .map_or(0, |(_, value)| value.parse().unwrap());

  let body_end = head_end + content_length;
  while buffer.len() < body_end {
    let bytes_read = stream.read(&mut chunk).await.unwrap();
    buffer.extend_from_slice(&chunk[..bytes_read]);
  }

  let bytes = buffer[head_end..body_end].to_vec();
  let body = String::from_utf8_lossy(&bytes).into_owned();
  (Recorded { method, path, headers, body, bytes }, stream)
}

async fn write_reply(mut stream: TcpStream, reply: Reply) {
  let status_line = format!("HTTP/1.1 {} X\r\n", reply.status);
  let fixed_headers = format!(
    "content-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
    reply.body.len()
  );
  let response =
    format!("{status_line}{fixed_headers}{}\r\n{}", reply.headers, reply.body);

  stream.write_all(response.as_bytes()).await.unwrap();
  stream.shutdown().await.unwrap();
}

fn welcome_email() -> SendEmail {
  SendEmail {
    from: Some("Acme <hi@acme.dev>".into()),
    to: vec!["ada@example.com".into()],
    subject: Some("Welcome".into()),
    html: Some("<p>Hi</p>".into()),
    ..SendEmail::default()
  }
}

fn rate_limited() -> Value {
  json!({
      "name": "rate_limit_exceeded",
      "status": 429,
      "message": "Too many requests",
      "errors": [],
      "request_id": "req_1"
  })
}

fn concurrent() -> Value {
  json!({
      "name": "concurrent_idempotent_requests",
      "status": 409,
      "message": "A request with this key is still running",
      "errors": [],
      "request_id": null
  })
}

fn server_error() -> Value {
  json!({
      "name": "unknown_error",
      "status": 500,
      "message": "Something went wrong. Please try again later.",
      "errors": [],
      "request_id": "req_2"
  })
}

#[tokio::test]
async fn sends_an_email_with_auth_and_idempotency_headers() {
  let mock = start_mock(vec![reply(202, json!({ "id": "eml_1" }))]).await;

  let queued = mock
    .client()
    .emails()
    .idempotency_key("welcome-ada")
    .send(&welcome_email())
    .await
    .unwrap();
  assert_eq!(queued.id, "eml_1");

  let requests = mock.requests();
  let request = &requests[0];
  assert_eq!(request.method, "POST");
  assert_eq!(request.path, "/v1/emails");

  assert_eq!(request.header("authorization"), Some("Bearer ibx_test"));
  assert_eq!(request.header("idempotency-key"), Some("welcome-ada"));
  assert_eq!(request.header("content-type"), Some("application/json"));
  assert!(request.header("user-agent").unwrap().starts_with("inboxjot-rust/"));

  assert_eq!(
    request.json(),
    json!({
        "from": "Acme <hi@acme.dev>",
        "to": ["ada@example.com"],
        "subject": "Welcome",
        "html": "<p>Hi</p>"
    })
  );
}

#[tokio::test]
async fn omits_the_idempotency_header_without_a_key() {
  let canceled_reply = reply(200, json!({ "id": "eml_1", "status": "canceled" }));
  let mock = start_mock(vec![canceled_reply]).await;

  let canceled = mock.client().emails().cancel("eml_1").await.unwrap();
  assert_eq!(canceled.status, inboxjot::EmailStatus::Canceled);

  let request = &mock.requests()[0];
  assert_eq!(request.path, "/v1/emails/eml_1/cancel");
  assert_eq!(request.header("idempotency-key"), None);
  assert_eq!(request.body, "");
}

#[tokio::test]
async fn retries_a_concurrent_idempotent_request_with_the_same_key() {
  let mock = start_mock(vec![
    reply_with(409, "retry-after: 0\r\n", concurrent()),
    reply(202, json!({ "data": [{ "id": "eml_1" }, { "id": "eml_2" }] })),
  ])
  .await;

  let email = BatchEmail {
    from: Some("Acme <hi@acme.dev>".into()),
    to: vec!["ada@example.com".into()],
    subject: Some("Welcome".into()),
    html: Some("<p>Hi</p>".into()),
    ..BatchEmail::default()
  };
  let emails = [email.clone(), email];

  let batch = mock
    .client()
    .emails()
    .idempotency_key("k1")
    .batch(&emails)
    .await
    .unwrap();
  assert_eq!(batch.data.len(), 2);

  let requests = mock.requests();
  assert_eq!(requests.len(), 2);
  assert_eq!(requests[1].header("idempotency-key"), Some("k1"));
  assert_eq!(requests[0].body, requests[1].body);
}

#[tokio::test]
async fn retries_a_rate_limited_post() {
  let mock = start_mock(vec![
    reply_with(429, "retry-after: 0\r\n", rate_limited()),
    reply(202, json!({ "contact_id": "con_1" })),
  ])
  .await;

  let event = inboxjot::Event {
    name: "signed_up".into(),
    email: "ada@example.com".into(),
    ..inboxjot::Event::default()
  };

  let tracked = mock.client().events().send(&event).await.unwrap();
  assert_eq!(tracked.contact_id, "con_1");
  assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn stops_after_max_retries() {
  let mock = start_mock(vec![
    reply_with(429, "retry-after: 0\r\n", rate_limited()),
    reply_with(429, "retry-after: 0\r\n", rate_limited()),
  ])
  .await;

  let client = Inboxjot::builder("ibx_test")
    .base_url(&mock.url)
    .max_retries(1)
    .build();

  let error = client.emails().send(&welcome_email()).await.unwrap_err();
  assert_eq!(error.name(), Some(ErrorName::RateLimitExceeded));
  assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn does_not_retry_a_server_error_on_a_post_without_a_key() {
  let mock = start_mock(vec![reply(500, server_error())]).await;

  let error = mock.client().emails().send(&welcome_email()).await.unwrap_err();
  let Error::Api(error) = error else {
    panic!("expected an API error");
  };

  assert_eq!(error.name, ErrorName::UnknownError);
  assert_eq!(error.request_id.as_deref(), Some("req_2"));
  assert_eq!(mock.requests().len(), 1);
}

#[tokio::test]
async fn retries_a_server_error_on_a_get() {
  let delivered_email = json!({
      "id": "eml_1",
      "from": "hi@acme.dev",
      "to": ["ada@example.com"],
      "cc": [],
      "bcc": [],
      "subject": "Hi",
      "tags": [{ "name": "kind", "value": "receipt" }],
      "status": "delivered",
      "source": "api",
      "scheduled_at": null,
      "sent_at": "2026-10-10T09:00:00.000Z",
      "created_at": "2026-10-10T08:59:59.000Z"
  });
  let mock = start_mock(vec![
    reply_with(503, "retry-after: 0\r\n", "upstream down"),
    reply(200, delivered_email),
  ])
  .await;

  let email = mock.client().emails().get("eml_1").await.unwrap();

  assert_eq!(email.status, inboxjot::EmailStatus::Delivered);
  assert_eq!(email.source, inboxjot::EmailSource::Api);
  assert_eq!(email.tags[0].name, "kind");
  assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn retries_a_get_after_a_dropped_connection() {
  let mock = start_mock(vec![None, reply(200, json!({ "data": [] }))]).await;

  let list = mock.client().emails().list_attachments("eml_1").await.unwrap();

  assert!(list.data.is_empty());
  assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn retries_a_server_error_on_a_delete_without_a_key() {
  let mock = start_mock(vec![
    reply_with(503, "retry-after: 0\r\n", server_error()),
    reply(200, json!({ "status": "ok" })),
  ])
  .await;

  mock.client().topics().delete("top_1").await.unwrap();

  assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn retries_a_delete_after_a_dropped_connection() {
  let deleted = json!({ "status": "ok" });
  let mock = start_mock(vec![None, reply(200, deleted)]).await;

  mock.client().contacts().delete("con_1").await.unwrap();

  assert_eq!(mock.requests().len(), 2);
}

#[test]
fn reads_the_error_names_added_to_the_api() {
  let names = [
    ("unsupported_media_type", ErrorName::UnsupportedMediaType),
    ("service_unavailable", ErrorName::ServiceUnavailable),
    ("some_new_error", ErrorName::Unknown),
  ];

  for (text, name) in names {
    assert_eq!(serde_json::from_value::<ErrorName>(json!(text)).unwrap(), name);
  }
}

#[test]
fn keeps_the_api_key_out_of_debug_output() {
  let builder = Inboxjot::builder("ibx_secret");
  assert!(!format!("{builder:?}").contains("ibx_secret"));
  assert!(!format!("{:?}", builder.build()).contains("ibx_secret"));
}

#[tokio::test]
async fn maps_a_validation_error() {
  let validation_error = json!({
      "name": "validation_error",
      "status": 400,
      "message": "Invalid address: nope",
      "errors": [{ "message": "Invalid address: nope", "location": "to" }],
      "request_id": "req_3"
  });
  let mock = start_mock(vec![reply(400, validation_error)]).await;

  let error = mock.client().emails().send(&welcome_email()).await.unwrap_err();
  let Error::Api(error) = error else {
    panic!("expected an API error");
  };

  assert_eq!(error.name, ErrorName::ValidationError);
  assert_eq!(error.status, 400);
  assert_eq!(error.errors[0].location, "to");
  assert_eq!(error.to_string(), "Invalid address: nope (400 ValidationError)");
}

#[tokio::test]
async fn maps_an_error_body_that_is_not_json() {
  let mock = start_mock(vec![reply(502, "Bad Gateway")]).await;

  let error = mock.client().emails().send(&welcome_email()).await.unwrap_err();
  let Error::Api(error) = error else {
    panic!("expected an API error");
  };

  assert_eq!(error.name, ErrorName::Unknown);
  assert_eq!(error.status, 502);
  assert_eq!(error.message, "Bad Gateway");
}

#[tokio::test]
async fn updates_a_contact_by_email_and_clears_a_field() {
  let updated_contact = json!({
      "id": "con_1",
      "email": "ada+news@example.com",
      "first_name": "Ada",
      "last_name": null,
      "status": "subscribed",
      "language": "en",
      "properties": {},
      "segments": [],
      "topics": [{
          "id": "top_1",
          "name": "News",
          "status": "subscribed",
          "subscribed": true
      }],
      "created_at": "2026-10-10T09:00:00.000Z",
      "updated_at": "2026-10-10T09:00:00.000Z"
  });
  let mock = start_mock(vec![reply(200, updated_contact)]).await;

  let topic = inboxjot::TopicChoice {
    id: "top_1".into(),
    status: inboxjot::TopicStatus::Subscribed,
  };
  let update = UpdateContact {
    first_name: Some(Some("Ada".into())),
    last_name: Some(None),
    topics: Some(vec![topic]),
    ..UpdateContact::default()
  };

  let contact = mock
    .client()
    .contacts()
    .update("ada+news@example.com", &update)
    .await
    .unwrap();
  assert_eq!(contact.first_name.as_deref(), Some("Ada"));
  assert_eq!(contact.created_at, "2026-10-10T09:00:00.000Z");
  assert_eq!(contact.topics[0].status, Some(inboxjot::ContactStatus::Subscribed));

  let request = &mock.requests()[0];
  assert_eq!(request.method, "PATCH");
  assert_eq!(request.path, "/v1/contacts/ada%2Bnews@example.com");
  assert_eq!(
    request.json(),
    json!({
        "first_name": "Ada",
        "last_name": null,
        "topics": [{ "id": "top_1", "status": "subscribed" }]
    })
  );
}

#[tokio::test]
async fn lists_emails_with_query_params() {
  let email_page = json!({
      "data": [{
          "id": "eml_1",
          "from": "hi@acme.dev",
          "to": ["ada@example.com"],
          "cc": [],
          "bcc": [],
          "subject": "Hi",
          "tags": [],
          "status": "sent",
          "source": "campaign",
          "scheduled_at": null,
          "sent_at": "2026-10-10T09:00:00.000Z",
          "created_at": "2026-10-10T08:59:59.000Z"
      }],
      "next_cursor": "cur_2",
      "prev_cursor": null
  });
  let mock = start_mock(vec![reply(200, email_page)]).await;

  let params = inboxjot::ListEmails {
    status_op: Some(inboxjot::EmailStatusOp::Not),
    source: Some(inboxjot::EmailSource::Campaign),
    sort: Some(inboxjot::EmailSort::To),
    direction: Some(inboxjot::SortDirection::Desc),
    per_page: Some(10),
    ..inboxjot::ListEmails::default()
  };

  let page = mock.client().emails().list(&params).await.unwrap();
  assert_eq!(page.data[0].source, inboxjot::EmailSource::Campaign);
  assert_eq!(page.next_cursor.as_deref(), Some("cur_2"));

  let request = &mock.requests()[0];
  assert_eq!(request.method, "GET");
  assert_eq!(
    request.path,
    "/v1/emails?status_op=not&source=campaign&sort=to&direction=desc&per_page=10"
  );
}

#[tokio::test]
async fn lists_contacts_with_query_params() {
  let contact_page = json!({
      "data": [{
          "id": "con_1",
          "email": "ada@example.com",
          "first_name": "Ada",
          "last_name": null,
          "properties": {},
          "status": "subscribed",
          "is_suppressed": false,
          "segments": [{ "id": "seg_1", "name": "Beta" }],
          "last_activity": { "type": "opened", "at": "2026-10-10T09:00:00.000Z" },
          "created_at": "2026-10-10T09:00:00.000Z"
      }],
      "total": 1,
      "next_cursor": null,
      "prev_cursor": null
  });
  let mock = start_mock(vec![reply(200, contact_page)]).await;

  let params = inboxjot::ListContacts {
    sort: Some(inboxjot::ContactSort::LastActivity),
    status: Some(inboxjot::ContactStatus::Subscribed),
    segment_id: Some("seg_1".into()),
    ..inboxjot::ListContacts::default()
  };

  let page = mock.client().contacts().list(&params).await.unwrap();
  assert_eq!(page.total, 1);
  assert_eq!(page.data[0].segments[0].name, "Beta");
  assert_eq!(page.data[0].last_activity.as_ref().unwrap().kind, "opened");

  let request = &mock.requests()[0];
  assert_eq!(request.method, "GET");
  assert_eq!(
    request.path,
    "/v1/contacts?sort=last_activity&status=subscribed&segment_id=seg_1"
  );
}

#[tokio::test]
async fn reschedules_an_email() {
  let body = json!({ "id": "eml_1", "scheduled_at": AT });
  let mock = start_mock(vec![reply(200, body)]).await;

  let update = UpdateEmail { scheduled_at: AT.into() };
  mock.client().emails().update("eml_1", &update).await.unwrap();

  let request = &mock.requests()[0];
  assert_eq!(request.method, "PATCH");
  assert_eq!(request.path, "/v1/emails/eml_1");
  assert_eq!(request.body, format!(r#"{{"scheduled_at":"{AT}"}}"#));
}

#[tokio::test]
async fn erases_a_contact() {
  let mock = start_mock(vec![reply(202, json!({ "status": "queued" }))]).await;

  let erasure = EraseContact { email: "ada@example.com".into() };
  let erased = mock.client().contacts().erase(&erasure).await.unwrap();
  assert_eq!(erased.status, ErasureStatus::Queued);

  let request = &mock.requests()[0];
  assert_eq!(request.method, "POST");
  assert_eq!(request.path, "/v1/contacts/erase");
  assert_eq!(request.json(), json!({ "email": "ada@example.com" }));
}

const AT: &str = "2026-10-21T09:00:00.000Z";

fn replies(bodies: Vec<Value>) -> Vec<Option<Reply>> {
  bodies.into_iter().map(|body| reply(200, body)).collect()
}

fn assert_routes(mock: &Mock, expected: &[(&str, &str)]) {
  let requests = mock.requests();
  let actual: Vec<(&str, &str)> = requests
    .iter()
    .map(|request| (request.method.as_str(), request.path.as_str()))
    .collect();

  assert_eq!(actual, expected);
}

fn status_ok() -> Value {
  json!({ "status": "ok" })
}

fn version() -> Value {
  json!({
      "id": "ver_1",
      "reason": "manual",
      "name": null,
      "editors": [{ "kind": "api", "id": "key_1", "name": "CI key" }],
      "created_at": AT
  })
}

fn version_page() -> Value {
  json!({ "data": [version()], "total": 1, "prev_cursor": null, "next_cursor": null })
}

fn version_detail() -> Value {
  json!({
      "id": "ver_1",
      "reason": "published",
      "name": "Launch",
      "editors": [],
      "from": "Acme <hi@acme.dev>",
      "reply_to": null,
      "subject": "Hi",
      "preheader": "",
      "html": "<p>Hi</p>",
      "text": "Hi",
      "content": { "type": "doc", "content": [] },
      "created_at": AT
  })
}

fn campaign() -> Value {
  json!({
      "id": "cpg_1",
      "name": "October",
      "status": "draft",
      "from": "Acme <hi@acme.dev>",
      "reply_to": null,
      "subject": "News",
      "preheader": "",
      "html": null,
      "text": null,
      "template_id": null,
      "template": null,
      "include_segment_ids": ["seg_1"],
      "include_segments": [{ "id": "seg_1", "name": "Customers" }],
      "exclude_segment_ids": [],
      "exclude_segments": [],
      "topic_id": "top_1",
      "topic": null,
      "audience_count": null,
      "queued_count": 0,
      "skipped_count": 0,
      "paused_reason": null,
      "failure_reason": null,
      "scheduled_at": null,
      "started_at": null,
      "completed_at": null,
      "warmup_next_limit": null,
      "created_at": AT,
      "updated_at": AT
  })
}

fn campaign_page() -> Value {
  let item = json!({
      "id": "cpg_1",
      "name": "October",
      "status": "paused",
      "subject": "News",
      "include_segments": [],
      "exclude_segments": [],
      "topic": null,
      "audience_count": null,
      "queued_count": 3,
      "skipped_count": 1,
      "paused_reason": "daily_limit",
      "scheduled_at": null,
      "started_at": AT,
      "completed_at": null,
      "created_at": AT,
      "updated_at": AT,
      "stats": null
  });

  json!({ "data": [item], "prev_cursor": null, "next_cursor": "cur_2" })
}

#[tokio::test]
async fn maps_campaign_methods_to_routes() {
  let mock = start_mock(replies(vec![
    campaign_page(),
    campaign(),
    campaign(),
    campaign(),
    status_ok(),
    campaign(),
    campaign(),
    campaign(),
    json!({ "ids": ["eml_1"] }),
    version_page(),
    version(),
    version_detail(),
    version(),
    campaign(),
  ]))
  .await;

  let client = mock.client();
  let campaigns = client.campaigns();
  let input = inboxjot::CampaignInput::default();
  let versions = inboxjot::ListVersions {
    filter: Some(inboxjot::VersionFilter::Saved),
    ..Default::default()
  };
  let test = SendTestCampaign { to: vec!["ada@example.com".into()] };
  let params = inboxjot::ListCampaigns {
    status: Some("draft,sent".into()),
    sort: Some(inboxjot::CampaignSort::Name),
    ..Default::default()
  };

  campaigns.list(&params).await.unwrap();
  campaigns.create(&input).await.unwrap();
  campaigns.get("cpg_1").await.unwrap();
  campaigns.update("cpg_1", &input).await.unwrap();
  campaigns.delete("cpg_1").await.unwrap();
  campaigns.send("cpg_1", &SendCampaign::default()).await.unwrap();
  let scheduled = SendCampaign { scheduled_at: Some(AT.into()) };
  campaigns.send("cpg_1", &scheduled).await.unwrap();
  campaigns.cancel("cpg_1").await.unwrap();
  campaigns.send_test("cpg_1", &test).await.unwrap();
  campaigns.list_versions("cpg_1", &versions).await.unwrap();
  let version = CreateVersion { name: "Launch".into() };
  campaigns.create_version("cpg_1", &version).await.unwrap();
  campaigns.get_version("cpg_1", "ver_1").await.unwrap();
  let unnamed = UpdateVersion::default();
  campaigns.update_version("cpg_1", "ver_1", &unnamed).await.unwrap();
  campaigns.restore_version("cpg_1", "ver_1").await.unwrap();

  assert_routes(
    &mock,
    &[
      ("GET", "/v1/campaigns?status=draft%2Csent&sort=name"),
      ("POST", "/v1/campaigns"),
      ("GET", "/v1/campaigns/cpg_1"),
      ("PATCH", "/v1/campaigns/cpg_1"),
      ("DELETE", "/v1/campaigns/cpg_1"),
      ("POST", "/v1/campaigns/cpg_1/send"),
      ("POST", "/v1/campaigns/cpg_1/send"),
      ("POST", "/v1/campaigns/cpg_1/cancel"),
      ("POST", "/v1/campaigns/cpg_1/test"),
      ("GET", "/v1/campaigns/cpg_1/versions?filter=saved"),
      ("POST", "/v1/campaigns/cpg_1/versions"),
      ("GET", "/v1/campaigns/cpg_1/versions/ver_1"),
      ("PATCH", "/v1/campaigns/cpg_1/versions/ver_1"),
      ("POST", "/v1/campaigns/cpg_1/versions/ver_1/restore"),
    ],
  );

  let requests = mock.requests();
  assert_eq!(requests[5].body, "{}");
  assert_eq!(requests[6].json(), json!({ "scheduled_at": AT }));
  assert_eq!(requests[8].json(), json!({ "to": ["ada@example.com"] }));
  assert_eq!(requests[10].json(), json!({ "name": "Launch" }));
  assert_eq!(requests[10].body, r#"{"name":"Launch"}"#);
  assert_eq!(requests[12].body, r#"{"name":null}"#);
}

#[test]
fn decodes_campaigns() {
  let campaign: inboxjot::Campaign = serde_json::from_value(campaign()).unwrap();
  assert_eq!(campaign.status, inboxjot::CampaignStatus::Draft);
  assert_eq!(campaign.topic_id, "top_1");
  assert_eq!(campaign.html, None);
  assert_eq!(campaign.include_segments[0].name, "Customers");

  let page: inboxjot::CampaignPage = serde_json::from_value(campaign_page()).unwrap();
  assert_eq!(page.data[0].paused_reason, Some(inboxjot::CampaignPauseReason::DailyLimit));
  assert_eq!(page.data[0].stats, None);

  let mut new_status = self::campaign();
  new_status["status"] = json!("archived");
  let campaign: inboxjot::Campaign = serde_json::from_value(new_status).unwrap();
  assert_eq!(campaign.status, inboxjot::CampaignStatus::Unknown);

  let page: inboxjot::DocumentVersionPage =
    serde_json::from_value(version_page()).unwrap();
  assert_eq!(page.data[0].reason, inboxjot::DocumentVersionReason::Manual);
  assert_eq!(page.data[0].editors[0].kind, inboxjot::DocumentVersionEditorKind::Api);

  let detail: inboxjot::DocumentVersionDetail =
    serde_json::from_value(version_detail()).unwrap();
  assert_eq!(detail.name.as_deref(), Some("Launch"));
  assert_eq!(detail.reply_to, None);
}

fn template() -> Value {
  json!({
      "id": "tpl_1",
      "name": "Welcome",
      "from": "Acme <hi@acme.dev>",
      "reply_to": null,
      "subject": "Welcome",
      "preheader": "",
      "html": "<p>Hi</p>",
      "text": "Hi",
      "published_version_id": null,
      "has_unpublished_changes": true,
      "created_at": AT,
      "updated_at": AT
  })
}

fn template_page() -> Value {
  let item = json!({
      "id": "tpl_1",
      "name": "Welcome",
      "from": "Acme <hi@acme.dev>",
      "subject": "Welcome",
      "preheader": "",
      "stats": { "sent": 10, "opened": 5, "clicked": 2 },
      "used_in": { "count": 0, "first": null },
      "created_at": AT,
      "updated_at": AT
  });

  json!({ "data": [item], "prev_cursor": null, "next_cursor": null })
}

#[tokio::test]
async fn maps_template_methods_to_routes() {
  let mock = start_mock(replies(vec![
    template_page(),
    template(),
    template(),
    template(),
    status_ok(),
    template(),
    version_page(),
    version(),
    version_detail(),
    version(),
    template(),
  ]))
  .await;

  let client = mock.client();
  let templates = client.templates();
  let versions = inboxjot::ListVersions::default();
  let create = inboxjot::CreateTemplate {
    name: "Welcome".into(),
    starter_id: Some("welcome".into()),
    ..Default::default()
  };
  let update = inboxjot::UpdateTemplate { reply_to: Some(None), ..Default::default() };
  let params = inboxjot::ListTemplates {
    sort: Some(inboxjot::TemplateSort::Updated),
    ..Default::default()
  };

  templates.list(&params).await.unwrap();
  templates.create(&create).await.unwrap();
  templates.get("tpl_1").await.unwrap();
  templates.update("tpl_1", &update).await.unwrap();
  templates.delete("tpl_1").await.unwrap();
  templates.publish("tpl_1").await.unwrap();
  templates.list_versions("tpl_1", &versions).await.unwrap();
  let version = CreateVersion { name: "Launch".into() };
  templates.create_version("tpl_1", &version).await.unwrap();
  templates.get_version("tpl_1", "ver_1").await.unwrap();
  let named = UpdateVersion { name: Some("Final".into()) };
  templates.update_version("tpl_1", "ver_1", &named).await.unwrap();
  templates.restore_version("tpl_1", "ver_1").await.unwrap();

  assert_routes(
    &mock,
    &[
      ("GET", "/v1/templates?sort=updated"),
      ("POST", "/v1/templates"),
      ("GET", "/v1/templates/tpl_1"),
      ("PATCH", "/v1/templates/tpl_1"),
      ("DELETE", "/v1/templates/tpl_1"),
      ("POST", "/v1/templates/tpl_1/publish"),
      ("GET", "/v1/templates/tpl_1/versions"),
      ("POST", "/v1/templates/tpl_1/versions"),
      ("GET", "/v1/templates/tpl_1/versions/ver_1"),
      ("PATCH", "/v1/templates/tpl_1/versions/ver_1"),
      ("POST", "/v1/templates/tpl_1/versions/ver_1/restore"),
    ],
  );

  let requests = mock.requests();
  assert_eq!(requests[1].json(), json!({ "name": "Welcome", "starter_id": "welcome" }));
  assert_eq!(requests[3].json(), json!({ "reply_to": null }));
  assert_eq!(requests[9].json(), json!({ "name": "Final" }));
}

#[test]
fn decodes_templates() {
  let template: inboxjot::Template = serde_json::from_value(template()).unwrap();
  assert_eq!(template.published_version_id, None);
  assert!(template.has_unpublished_changes);

  let page: inboxjot::TemplatePage = serde_json::from_value(template_page()).unwrap();
  assert_eq!(page.data[0].stats.sent, 10);
  assert_eq!(page.data[0].used_in.first, None);
}

fn domain() -> Value {
  json!({
      "id": "dom_1",
      "name": "acme.dev",
      "mail_from_domain": "send.acme.dev",
      "status": "pending",
      "records": [
          {
              "kind": "dkim",
              "type": "TXT",
              "name": "inboxjot._domainkey.acme.dev",
              "value": "v=DKIM1",
              "status": "pending"
          },
          {
              "kind": "return_path",
              "type": "MX",
              "name": "send.acme.dev",
              "value": "feedback.example.com",
              "priority": 10,
              "status": "temporary_failure"
          }
      ],
      "region": "us-east-1",
      "tls_policy": "optional",
      "receiving_enabled": false,
      "receiving_target": "subdomain",
      "receiving_status": "not_started",
      "receiving_found_mx": null,
      "receiving_checked_at": null,
      "receiving_verified_at": null,
      "open_tracking": true,
      "click_tracking": true,
      "tracking_domain": null,
      "tracking_status": "not_started",
      "tracking_records": [],
      "tracking_verified_at": null,
      "mailing_address": null,
      "verified_at": null,
      "created_at": AT,
      "updated_at": AT,
      "sent_last_30_days": 0,
      "bounced_last_30_days": 0,
      "is_default": true,
      "receiving": { "host": "inbound.acme.dev", "mx": "mx.aws", "shared_address": null },
      "warmup": {
          "enabled": false,
          "started_on": null,
          "day": null,
          "limit": null,
          "next_limit": null,
          "sent_today": 0,
          "full_volume_on": null
      }
  })
}

fn domain_page() -> Value {
  let item = json!({
      "id": "dom_1",
      "name": "acme.dev",
      "status": "verified",
      "pending_record_kinds": ["dmarc"],
      "region": "us-east-1",
      "is_default": true,
      "open_tracking": true,
      "click_tracking": false,
      "sent_last_30_days": 48210,
      "verified_at": null,
      "created_at": AT
  });

  json!({ "data": [item], "prev_cursor": null, "next_cursor": null })
}

#[tokio::test]
async fn maps_domain_methods_to_routes() {
  let mock =
    start_mock(replies(vec![domain_page(), domain(), domain(), status_ok(), domain()]))
      .await;

  let client = mock.client();
  let domains = client.domains();
  let params = inboxjot::PageParams { per_page: Some(2), ..Default::default() };
  let create = inboxjot::CreateDomain { name: "acme.dev".into(), ..Default::default() };

  domains.list(&params).await.unwrap();
  domains.create(&create).await.unwrap();
  domains.get("dom_1").await.unwrap();
  domains.delete("dom_1").await.unwrap();
  domains.verify("dom_1").await.unwrap();

  assert_routes(
    &mock,
    &[
      ("GET", "/v1/domains?per_page=2"),
      ("POST", "/v1/domains"),
      ("GET", "/v1/domains/dom_1"),
      ("DELETE", "/v1/domains/dom_1"),
      ("POST", "/v1/domains/dom_1/verify"),
    ],
  );
  assert_eq!(mock.requests()[1].json(), json!({ "name": "acme.dev" }));
}

#[test]
fn decodes_domains() {
  let domain: inboxjot::Domain = serde_json::from_value(domain()).unwrap();
  assert_eq!(domain.records[0].priority, None);
  assert_eq!(domain.records[1].priority, Some(10));
  assert_eq!(domain.records[1].record_type, "MX");
  assert_eq!(domain.records[1].status, inboxjot::DomainStatus::TemporaryFailure);
  assert_eq!(domain.tracking_status, inboxjot::TrackingStatus::NotStarted);
  assert_eq!(domain.warmup.limit, None);

  let page: inboxjot::DomainPage = serde_json::from_value(domain_page()).unwrap();
  assert_eq!(page.data[0].pending_record_kinds, [inboxjot::RecordKind::Dmarc]);
}

fn image() -> Value {
  json!({
      "id": "img_1",
      "file_name": "hero.png",
      "url": null,
      "content_type": "image/png",
      "size_bytes": 412_000,
      "width": 1200,
      "height": 630,
      "creator_name": null,
      "created_at": AT
  })
}

fn image_page() -> Value {
  json!({ "data": [image()], "total": 1, "prev_cursor": null, "next_cursor": null })
}

const PNG: [u8; 10] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2];

async fn start_upload_mock(put: Option<Reply>, create: Option<Reply>) -> Mock {
  start_mock_with(|url| {
    let target =
      json!({ "upload_url": format!("{url}/put?sig=1"), "key": "image-uploads/1" });
    vec![reply(200, target), put, create]
  })
  .await
}

#[tokio::test]
async fn uploads_an_image() {
  let mock = start_upload_mock(reply(200, ""), reply(201, image())).await;

  let upload = UploadImage {
    file: PNG.to_vec(),
    file_name: Some("logo.png".into()),
  };
  let image = mock.client().images().upload(&upload).await.unwrap();

  assert_eq!(image.id, "img_1");
  assert_routes(
    &mock,
    &[("POST", "/v1/images/presign"), ("PUT", "/put?sig=1"), ("POST", "/v1/images")],
  );

  let requests = mock.requests();
  assert_eq!(requests[0].json(), json!({ "content_type": "image/png", "size": 10 }));

  let put = &requests[1];
  assert_eq!(put.header("content-type"), Some("image/png"));
  assert_eq!(put.header("authorization"), None);
  assert_eq!(put.bytes, PNG);

  assert_eq!(
    requests[2].json(),
    json!({ "key": "image-uploads/1", "file_name": "logo.png" })
  );
}

#[tokio::test]
async fn detects_jpeg_and_gif() {
  let cases: [(&[u8], &str); 2] = [
    (&[0xFF, 0xD8, 0xFF, 0xE0], "image/jpeg"),
    (&[0x47, 0x49, 0x46, 0x38, 0x39, 0x61], "image/gif"),
  ];

  for (bytes, content_type) in cases {
    let mock = start_upload_mock(reply(200, ""), reply(201, image())).await;
    let upload = UploadImage { file: bytes.to_vec(), file_name: None };
    mock.client().images().upload(&upload).await.unwrap();

    let requests = mock.requests();
    assert_eq!(requests[0].json()["content_type"], content_type);
    assert_eq!(requests[1].header("content-type"), Some(content_type));
    assert_eq!(requests[2].json(), json!({ "key": "image-uploads/1" }));
  }
}

#[tokio::test]
async fn refuses_a_file_that_is_not_an_image() {
  let mock = start_mock(Vec::new()).await;

  let upload = UploadImage { file: vec![1, 2, 3], file_name: None };
  let error = mock.client().images().upload(&upload).await.unwrap_err();

  assert!(matches!(error, Error::UnsupportedImage));
  assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn refuses_an_idempotency_key_on_upload() {
  let mock = start_mock(Vec::new()).await;

  let client = mock.client();
  let upload = UploadImage { file: PNG.to_vec(), file_name: None };
  let error = client
    .images()
    .idempotency_key("k1")
    .upload(&upload)
    .await
    .unwrap_err();

  assert!(matches!(error, Error::UnsupportedIdempotencyKey));
  assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn fails_when_storage_rejects_the_upload() {
  let body = "<Error>SignatureDoesNotMatch</Error>";
  let mock = start_upload_mock(reply(403, body), None).await;

  let upload = UploadImage { file: PNG.to_vec(), file_name: None };
  let error = mock.client().images().upload(&upload).await.unwrap_err();

  let Error::Api(error) = error else { panic!("expected an API error, got {error:?}") };
  assert_eq!(error.status, 403);
  assert_eq!(error.message, "The image upload failed with status 403");
  assert_eq!(mock.requests().len(), 2);
}

#[tokio::test]
async fn maps_image_methods_to_routes() {
  let mock = start_mock(replies(vec![image_page(), image(), status_ok()])).await;

  let client = mock.client();
  let images = client.images();
  let params = inboxjot::ListImages {
    kind: Some(inboxjot::ImageType::Png),
    ..Default::default()
  };

  images.list(&params).await.unwrap();
  images.get("img_1").await.unwrap();
  images.delete("img_1").await.unwrap();

  assert_routes(
    &mock,
    &[
      ("GET", "/v1/images?type=png"),
      ("GET", "/v1/images/img_1"),
      ("DELETE", "/v1/images/img_1"),
    ],
  );
}

#[test]
fn decodes_images() {
  let page: inboxjot::ImagePage = serde_json::from_value(image_page()).unwrap();
  assert_eq!(page.total, 1);
  assert_eq!(page.data[0].url, None);
  assert_eq!(page.data[0].size_bytes, 412_000);
}

fn segment() -> Value {
  json!({
      "id": "seg_1",
      "name": "Active",
      "description": null,
      "type": "dynamic",
      "conditions": {
          "match": "all",
          "rules": [
              { "field": "email", "operator": "is_set" },
              {
                  "field": "email.opened",
                  "operator": "at_least",
                  "value": { "count": 2, "days": 30 }
              },
              { "field": "property.plan", "operator": "is", "value": "pro" }
          ]
      },
      "contact_count": null,
      "counted_at": null,
      "created_at": AT,
      "updated_at": AT
  })
}

fn segment_page() -> Value {
  let item = json!({
      "id": "seg_1",
      "name": "Beta",
      "description": null,
      "type": "static",
      "contact_count": 12,
      "counted_at": null,
      "created_at": AT,
      "updated_at": AT
  });

  json!({ "data": [item], "total": 1, "prev_cursor": null, "next_cursor": null })
}

fn segment_member_page() -> Value {
  let member = json!({
      "id": "con_1",
      "email": "ada@example.com",
      "first_name": null,
      "last_name": null,
      "status": "pending",
      "joined_at": AT
  });

  json!({ "data": [member], "prev_cursor": null, "next_cursor": null })
}

#[tokio::test]
async fn maps_segment_methods_to_routes() {
  let mock = start_mock(replies(vec![
    segment_page(),
    segment(),
    segment(),
    segment(),
    status_ok(),
    segment_member_page(),
    json!({ "added": 1 }),
    json!({ "removed": 1 }),
  ]))
  .await;

  let client = mock.client();
  let segments = client.segments();
  let contact_ids = SegmentMembers { contact_ids: vec!["con_1".into()] };
  let params = inboxjot::ListSegments {
    kind: Some(inboxjot::SegmentType::Dynamic),
    sort: Some(inboxjot::SegmentSort::Contacts),
    ..Default::default()
  };
  let create = inboxjot::CreateSegment {
    name: "Active".into(),
    kind: inboxjot::SegmentType::Dynamic,
    conditions: Some(inboxjot::SegmentConditions {
      matching: inboxjot::SegmentMatch::Any,
      rules: vec![inboxjot::SegmentRule {
        field: "email".into(),
        operator: inboxjot::SegmentOperator::IsSet,
        value: None,
      }],
    }),
    ..Default::default()
  };
  let update = inboxjot::UpdateSegment { description: Some(None), ..Default::default() };
  let members = inboxjot::ListSegmentMembers::default();

  segments.list(&params).await.unwrap();
  segments.create(&create).await.unwrap();
  segments.get("seg_1").await.unwrap();
  segments.update("seg_1", &update).await.unwrap();
  segments.delete("seg_1").await.unwrap();
  segments.list_members("seg_1", &members).await.unwrap();
  segments.add_members("seg_1", &contact_ids).await.unwrap();
  let removed = segments.remove_members("seg_1", &contact_ids).await.unwrap();

  assert_eq!(removed.removed, 1);
  assert_routes(
    &mock,
    &[
      ("GET", "/v1/segments?type=dynamic&sort=contacts"),
      ("POST", "/v1/segments"),
      ("GET", "/v1/segments/seg_1"),
      ("PATCH", "/v1/segments/seg_1"),
      ("DELETE", "/v1/segments/seg_1"),
      ("GET", "/v1/segments/seg_1/members"),
      ("POST", "/v1/segments/seg_1/members"),
      ("DELETE", "/v1/segments/seg_1/members"),
    ],
  );

  let requests = mock.requests();
  assert_eq!(
    requests[1].json(),
    json!({
        "name": "Active",
        "type": "dynamic",
        "conditions": {
            "match": "any",
            "rules": [{ "field": "email", "operator": "is_set" }]
        }
    })
  );
  assert_eq!(requests[3].json(), json!({ "description": null }));
  assert_eq!(requests[6].body, r#"{"contact_ids":["con_1"]}"#);
  assert_eq!(requests[7].body, r#"{"contact_ids":["con_1"]}"#);
  assert_eq!(requests[7].header("content-type"), Some("application/json"));
}

#[test]
fn decodes_segments() {
  let segment: inboxjot::Segment = serde_json::from_value(segment()).unwrap();
  assert_eq!(segment.kind, inboxjot::SegmentType::Dynamic);
  assert_eq!(segment.contact_count, None);

  let conditions = segment.conditions.unwrap();
  assert_eq!(conditions.matching, inboxjot::SegmentMatch::All);
  assert_eq!(conditions.rules[0].value, None);
  assert_eq!(conditions.rules[1].operator, inboxjot::SegmentOperator::AtLeast);
  assert_eq!(conditions.rules[1].value, Some(json!({ "count": 2, "days": 30 })));

  let page: inboxjot::SegmentPage = serde_json::from_value(segment_page()).unwrap();
  assert_eq!(page.data[0].kind, inboxjot::SegmentType::Static);
  assert_eq!(page.data[0].contact_count, Some(12));

  let members: inboxjot::SegmentMemberPage =
    serde_json::from_value(segment_member_page()).unwrap();
  assert_eq!(members.data[0].status, inboxjot::ContactStatus::Pending);
}

fn topic() -> Value {
  json!({
      "id": "top_1",
      "name": "Product updates",
      "description": null,
      "default_subscribed": true,
      "visibility": "public",
      "double_opt_in": false,
      "form_enabled": false,
      "sender_name": null,
      "sender_email": null,
      "redirect_url": null,
      "is_default": false,
      "counts": { "subscribed": 1, "pending": 0, "unsubscribed": 2 },
      "created_at": AT,
      "updated_at": AT
  })
}

fn topic_page() -> Value {
  json!({ "data": [topic()], "prev_cursor": null, "next_cursor": null })
}

#[tokio::test]
async fn maps_topic_methods_to_routes() {
  let mock =
    start_mock(replies(vec![topic_page(), topic(), topic(), topic(), status_ok()])).await;

  let client = mock.client();
  let topics = client.topics();
  let params = inboxjot::PageParams::default();
  let create = inboxjot::CreateTopic {
    name: "News".into(),
    visibility: Some(inboxjot::TopicVisibility::Private),
    ..Default::default()
  };
  let update = inboxjot::UpdateTopic { sender_name: Some(None), ..Default::default() };

  topics.list(&params).await.unwrap();
  topics.create(&create).await.unwrap();
  topics.get("top_1").await.unwrap();
  topics.update("top_1", &update).await.unwrap();
  topics.delete("top_1").await.unwrap();

  assert_routes(
    &mock,
    &[
      ("GET", "/v1/topics"),
      ("POST", "/v1/topics"),
      ("GET", "/v1/topics/top_1"),
      ("PATCH", "/v1/topics/top_1"),
      ("DELETE", "/v1/topics/top_1"),
    ],
  );

  let requests = mock.requests();
  assert_eq!(requests[1].json(), json!({ "name": "News", "visibility": "private" }));
  assert_eq!(requests[3].json(), json!({ "sender_name": null }));
}

#[test]
fn decodes_topics() {
  let page: inboxjot::TopicPage = serde_json::from_value(topic_page()).unwrap();
  let topic = &page.data[0];

  assert_eq!(topic.visibility, inboxjot::TopicVisibility::Public);
  assert_eq!(topic.sender_email, None);
  assert_eq!(topic.counts.unsubscribed, 2);
}

fn endpoint(extra: &Value) -> Value {
  let mut endpoint = json!({
      "id": "wh_1",
      "name": "Ops",
      "url": "https://acme.dev/hook",
      "events": ["email.bounced", "contact.deleted"],
      "status": "enabled",
      "disabled_reason": null,
      "created_at": AT,
      "updated_at": AT
  });

  for (key, value) in extra.as_object().unwrap() {
    endpoint[key] = value.clone();
  }

  endpoint
}

fn endpoint_page() -> Value {
  let item = endpoint(&json!({ "success_rate": null, "last_delivery_at": null }));
  json!({ "data": [item], "prev_cursor": null, "next_cursor": null })
}

fn endpoint_detail() -> Value {
  endpoint(&json!({
      "deliveries_24h": 3,
      "success_rate": 0.5,
      "avg_duration_ms": null,
      "retrying": 0,
      "recent_deliveries": [{
          "delivery_id": "dlv_1",
          "event_id": "evt_1",
          "event_type": "email.bounced",
          "endpoint_id": "wh_1",
          "email_id": "eml_1",
          "status": 500,
          "attempt": 2,
          "duration_ms": 120,
          "created_at": AT,
          "last_attempt_at": AT,
          "next_attempt_at": null
      }]
  }))
}

#[tokio::test]
async fn maps_webhook_methods_to_routes() {
  let mock = start_mock(replies(vec![
    endpoint_page(),
    endpoint(&json!({ "secret": "whsec_abc" })),
    endpoint_detail(),
    endpoint(&json!({})),
    status_ok(),
  ]))
  .await;

  let client = mock.client();
  let webhooks = client.webhooks();
  let params = inboxjot::PageParams::default();
  let create = inboxjot::CreateWebhookEndpoint {
    name: "Ops".into(),
    url: "https://acme.dev/hook".into(),
    events: vec![inboxjot::WebhookEventType::EmailBounced],
  };
  let update = inboxjot::UpdateWebhookEndpoint {
    status: Some(inboxjot::EndpointStatus::Disabled),
    ..Default::default()
  };

  webhooks.list(&params).await.unwrap();
  let created = webhooks.create(&create).await.unwrap();
  webhooks.get("wh_1").await.unwrap();
  webhooks.update("wh_1", &update).await.unwrap();
  webhooks.delete("wh_1").await.unwrap();

  assert_eq!(created.secret, "whsec_abc");
  assert_routes(
    &mock,
    &[
      ("GET", "/v1/webhooks"),
      ("POST", "/v1/webhooks"),
      ("GET", "/v1/webhooks/wh_1"),
      ("PATCH", "/v1/webhooks/wh_1"),
      ("DELETE", "/v1/webhooks/wh_1"),
    ],
  );

  let requests = mock.requests();
  assert_eq!(
    requests[1].json(),
    json!({
        "name": "Ops",
        "url": "https://acme.dev/hook",
        "events": ["email.bounced"]
    })
  );
  assert_eq!(requests[3].json(), json!({ "status": "disabled" }));
}

#[test]
fn decodes_webhooks() {
  let page: inboxjot::WebhookEndpointPage =
    serde_json::from_value(endpoint_page()).unwrap();
  assert_eq!(page.data[0].events[1], inboxjot::WebhookEventType::ContactDeleted);
  assert_eq!(page.data[0].success_rate, None);

  let detail: inboxjot::WebhookEndpointDetail =
    serde_json::from_value(endpoint_detail()).unwrap();
  assert_eq!(detail.success_rate, Some(0.5));
  assert_eq!(detail.avg_duration_ms, None);
  assert_eq!(detail.recent_deliveries[0].next_attempt_at, None);

  let new_event = endpoint(&json!({ "events": ["domain.verified"], "status": "paused" }));
  let endpoint: inboxjot::WebhookEndpoint = serde_json::from_value(new_event).unwrap();
  assert_eq!(endpoint.events[0], inboxjot::WebhookEventType::Unknown);
  assert_eq!(endpoint.status, inboxjot::EndpointStatus::Unknown);
}
