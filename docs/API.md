# Air gRPC API Reference

This document describes the gRPC surface exposed by Air (see Proto/air.proto for canonical definitions).

Service: `AirService`

Summary of RPCs:

- `Authenticate(AuthenticationRequest) returns (AuthenticationResponse)`
  - Purpose: perform user/service authentication and return a short-lived token.
  - Notes: token included in subsequent RPC metadata as `authorization: Bearer <token>`.

- Update operations:
  - `CheckForUpdates(UpdateCheckRequest) returns (UpdateCheckResponse)`
  - `DownloadUpdate(DownloadRequest) returns (DownloadResponse)`
  - `ApplyUpdate(ApplyUpdateRequest) returns (ApplyUpdateResponse)`
  - Purpose: full update lifecycle.
  - Errors: network failures surface via `error` field in the response; server uses gRPC status codes for unrecoverable conditions.

- Download operations:
  - `DownloadFile(DownloadRequest) returns (DownloadResponse)`
  - `DownloadStream(DownloadStreamRequest) returns (stream DownloadStreamResponse)`
  - Purpose: file downloads; streaming RPC used for large payloads.
  - Usage: clients may resume by repeating `DownloadStream` with same `request_id` if server supports resume; see `docs/CONFIGURATION.md` for server resume behavior.

- Indexing & file ops:
  - `IndexFiles`, `SearchFiles`, `GetFileInfo`
  - Purpose: filesystem indexing and search delegation to the background daemon.

- Status & metrics:
  - `GetStatus`, `HealthCheck`, `GetMetrics`
  - Purpose: monitoring and health checks.

- Resource management & configuration
  - `GetResourceUsage`, `SetResourceLimits`, `GetConfiguration`, `UpdateConfiguration`

Field-level notes:
- Most requests and responses include `request_id` (string). Use stable UUIDs to correlate client-side logs with server logs.
- Errors are communicated via two channels:
  1. The `error` string field in the response messages for expected/handled problems.
  2. gRPC status codes for transport-level or permission errors (e.g., `PermissionDenied`, `Unavailable`).

Authentication & Metadata usage example (grpcurl):

```bash
# Check health (no auth required if server allows local checks)
grpcurl -plaintext localhost:50053 air.AirService/HealthCheck

# Authenticate then call another RPC using returned token
TOKEN=$(grpcurl -plaintext -d '{"username":"me","password":"secret"}' localhost:50053 air.AirService/Authenticate | jq -r .token)
grpcurl -H "authorization: Bearer $TOKEN" -plaintext -d '{"request_id":"1","current_version":"0.1.0","channel":"stable"}' localhost:50053 air.AirService/CheckForUpdates
```

Rust client example (tonic generated client):

```rust
use air::air_service_client::AirServiceClient;
use air::AuthenticationRequest;

let mut client = AirServiceClient::connect("http://[::1]:50053").await?;
let resp = client.authenticate(AuthenticationRequest { request_id: "req-1".into(), username: "me".into(), password: "pw".into(), provider: "github".into() }).await?;
let token = resp.into_inner().token;

// Attach token to future requests via tonic::Request metadata
let mut req = tonic::Request::new(UpdateCheckRequest { request_id: "req-2".into(), current_version: "0.1.0".into(), channel: "stable".into() });
req.metadata_mut().insert("authorization", format!("Bearer {}", token).parse().unwrap());
let update = client.check_for_updates(req).await?;
```

Error handling guidance:
- Inspect both the gRPC status and the response `error` string.
- Retry idempotent operations (e.g., `CheckForUpdates`) with exponential backoff on `Unavailable`.
- For streaming downloads, treat missing chunks as transient network errors and attempt resume if supported.

For complete protobuf message definitions, see `Proto/air.proto`.
