# openauth-telemetry

Telemetry support for OpenAuth-RS.

## Status

This package is in experimental beta. Payload shape, detection behavior,
environment variables, and transport hooks may change before stable release.

## What It Provides

`openauth-telemetry` builds Better Auth-shaped telemetry payloads for Rust
hosts. Publishing is opt-in: without `OPENAUTH_TELEMETRY_ENDPOINT` or a custom
track function, the publisher is a hard no-op.

## Example

```rust
use openauth::{OpenAuthOptions, TelemetryOptions};
use openauth_telemetry::{create_telemetry, TelemetryContext, TelemetryEvent};
use serde_json::json;

let options = OpenAuthOptions::new()
    .base_url("https://app.example.com/api/auth")
    .telemetry(TelemetryOptions::new().enabled(true));

let publisher = create_telemetry(&options, TelemetryContext::default()).await;
publisher
    .publish(TelemetryEvent {
        event_type: "custom".to_owned(),
        anonymous_id: None,
        payload: json!({ "source": "app" }),
    })
    .await;
```

Telemetry does not send anything by default; the deployer controls the endpoint
or custom sink.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
