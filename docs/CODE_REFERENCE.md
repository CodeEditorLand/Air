# Code Reference — Public APIs

This document summarizes the main public crates, modules, and types that consumers and integrators will use.

Crate layout (high level)

- `air` (root crate)
  - `server` — gRPC server implementation and `AirService` wiring.
  - `auth` — authentication/token management and signer helpers.
  - `update` — update manager: check, download, verify, and apply.
  - `downloader` — resilient downloader with retry and resume support.
  - `config` — configuration loading and validation.
  - `metrics` — runtime metrics and instrumentation.

Public types and usage

- `Air::Config` — central configuration struct. Load via `Config::load(path)` or `Config::from_env()`.
- `Air::AirService` — server implementation type. Start server with `AirService::run(config)`.
- `UpdateManager` — programmatic API to check/download/apply updates. Example:

```rust
let cfg = Config::load("/etc/air/config.toml")?;
let mut mgr = UpdateManager::new(&cfg);
let info = mgr.check_for_updates("stable").await?;
if info.update_available {
    mgr.download_update(info.download_url).await?;
    mgr.apply_update(info.version).await?;
}
```

Generated gRPC client

Protobuf sources live in `Proto/air.proto` and are compiled by `build.rs`. Use the generated clients for programmatic access. Example (tonic client): see `docs/API.md`.

Error handling

- Public APIs return `Result<T, AirError>` where `AirError` encodes internal errors. Convert or map errors to gRPC statuses at the server boundary.
- For long running tasks, return progress via events/streams and include error messages and structured error codes where possible.

Examples and patterns

- Always use `request_id` when calling remotely to enable correlation.
- Prefer idempotent operations for retries (e.g., `CheckForUpdates`, `GetStatus`).

For in-depth documentation, review public module docs in the source code and the generated Rust docs when available (`cargo doc --manifest-path Element/Air/Cargo.toml`).
