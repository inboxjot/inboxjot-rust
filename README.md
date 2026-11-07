![Inboxjot](https://inboxjot.com/og.png)

<p align="center">The official Rust SDK for the Inboxjot email API. <br />Email for builders and their agents.</p>

<p align="center">
  <a href="https://crates.io/crates/inboxjot"><img src="https://inboxjot.com/badges/crates.svg?v=2" alt="crates.io" height="24" /></a>
  <a href="https://docs.rs/inboxjot"><img src="https://inboxjot.com/badges/docs-rs.svg?v=2" alt="docs.rs" height="24" /></a>
  <a href="./LICENSE"><img src="https://inboxjot.com/badges/license.svg?v=2" alt="License" height="24" /></a>
  <a href="https://inboxjot.com/docs"><img src="https://inboxjot.com/badges/docs.svg?v=2" alt="Docs" height="24" /></a>
</p>

Send email, manage contacts, campaigns, templates and domains, and verify webhooks from your Rust app. The client is async, runs on Tokio and uses `reqwest` with rustls.

### Quick Start

```toml
[dependencies]
inboxjot = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use inboxjot::{Inboxjot, SendEmail};

let client = Inboxjot::new("ibx_...");

let queued = client
  .emails()
  .send(&SendEmail {
    from: Some("Acme <hi@acme.dev>".into()),
    to: vec!["jane@example.com".into()],
    subject: Some("Welcome".into()),
    html: Some("<p>Hi Jane</p>".into()),
    ..Default::default()
  })
  .await?;
```

`Inboxjot::from_env()` reads `INBOXJOT_API_KEY`. `Inboxjot::builder(key)` sets `base_url`, `timeout`, `max_retries` and `http_client`. The client is cheap to clone, and clones share one connection pool.

### Resources

```rust
client.contacts().create(&CreateContact { email: "jane@example.com".into(), ..Default::default() }).await?;
client.contacts().update("jane@example.com", &UpdateContact { last_name: Some(None), ..Default::default() }).await?;
client.contacts().erase(&EraseContact { email: "jane@example.com".into() }).await?;
client.events().send(&Event { name: "signed_up".into(), email: "jane@example.com".into(), data: Default::default() }).await?;

client.campaigns().send("cmp_...", &SendCampaign { scheduled_at: Some("2026-10-20T09:00:00Z".into()) }).await?;
client.templates().publish("tpl_...").await?;
client.segments().add_members("seg_...", &SegmentMembers { contact_ids: vec!["con_...".into()] }).await?;
```

Every method takes path ids first, then one params struct by reference. `contacts().get`, `update` and `delete` take a contact id or the contact's email. In update structs, `None` leaves a field as it is and `Some(None)` clears it.

An image that is already online goes straight into the content as `<img src="https://…">`. To host a PNG, JPEG or GIF of up to 5 MB on Inboxjot, upload it:

```rust
let image = client
  .images()
  .upload(&UploadImage { file: std::fs::read("logo.png")?, file_name: Some("logo.png".into()) })
  .await?;
```

The client covers the whole public API: `emails()`, `contacts()`, `events()`, `campaigns()`, `templates()`, `domains()`, `images()`, `segments()`, `topics()` and `webhooks()`. The [API reference](https://inboxjot.com/docs) lists every endpoint and field.

### Pagination

Lists page with a cursor. Pass `next_cursor` back as `after`. It is `None` on the last page.

```rust
let mut params = ListContacts { per_page: Some(100), ..Default::default() };
loop {
  let page = client.contacts().list(&params).await?;
  params.after = page.next_cursor;
  if params.after.is_none() {
    break;
  }
}
```

### Idempotency and Retries

Set an idempotency key on the resource before the call:

```rust
client.emails().idempotency_key(format!("welcome-{user_id}")).send(&email).await?;
```

The client retries with exponential backoff and honors `Retry-After`. A `429` always retries. A `5xx`, a timeout or a network error retries only for a `GET`, a `DELETE` or a request with an idempotency key, so an email is never sent twice. A `Retry-After` over 60 seconds, such as the daily sending limit, goes straight to the caller. `images().upload` returns `Error::UnsupportedIdempotencyKey` for a key, because each attempt gets a new upload URL.

### Errors

An API error is `Error::Api` with a typed `name`, plus `status`, `message`, `errors` and `request_id`.

```rust
match client.emails().get("eml_...").await {
  Err(Error::Api(error)) if error.name == ErrorName::NotFound => {}
  Err(error) => return Err(error.into()),
  Ok(email) => println!("{:?}", email.status),
}
```

### Verify Webhooks

Inboxjot signs webhooks with the [Standard Webhooks](https://www.standardwebhooks.com) spec. Verify the raw body before you parse it:

```rust
use inboxjot::webhooks::{self, WebhookEventData};

let event = webhooks::verify("whsec_...", &headers, &body)?;
if let WebhookEventData::EmailBounced(data) = event.data {
  println!("{} {:?}", data.email.email_id, data.bounce.kind);
}
```

`headers` is an `http::HeaderMap`, the type axum, hyper and reqwest use. A bad signature, a missing header or a timestamp more than 5 minutes off returns an error. The `webhooks` feature is on by default; turn it off with `default-features = false` to drop `hmac`, `sha2` and `base64`.

### Contributing

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` before you open a pull request.

### License

[MIT](./LICENSE) &copy; [Arik Chakma](https://x.com/imarikchakma)
