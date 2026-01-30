# gRPC / Vine Protocol Notes

Air exposes a gRPC service over loopback. The implementation uses the `tonic` / Rust ecosystem and follows these conventions.

Default transport
- Address: `localhost` / loopback only by default.
- Port: `50053` (configurable via configuration file)
- TLS: optional. When enabled, server expects `server.crt` (cert) and `server.key` (private key).

Metadata and authentication
- Authenticated RPCs require an `authorization` metadata header: `authorization: Bearer <token>`.
- Tokens are issued by `Authenticate` and are expected to be short-lived.
- Mutual TLS (mTLS) is supported by enabling `tls.client_auth` in configuration.

Timeouts and keepalive
- Clients should use conservative deadlines for calls that may block (e.g., downloads).
- Server enforces an overall request timeout; long-running operations (download, update) return progress events or use streaming RPCs.

Streaming
- `DownloadStream` is a server-streaming RPC that yields binary chunks.
- Clients must process chunks in order and can request resume logic by issuing a new request with the same `request_id` and appropriate `Range` headers (if server supports range resume).

Error semantics
- Use gRPC status codes for transport and permission errors.
- Business-level errors are returned in the message `error` fields.

Graceful shutdown and upgrade
- Air supports graceful shutdown signals and will finish inflight jobs where possible before exiting.
- During upgrades, a supervising process (e.g., systemd) should stop `air`, replace the binary, then start again. The service persists critical state under `/var/lib/air` by default.

Observability
- gRPC interceptors emit structured logs and metrics. Use `GetMetrics` for runtime metrics.

Security
- Always enable TLS in production and store keys in protected locations (e.g., `/etc/air/certs`).
- Restrict network exposure: bind to loopback or use unix sockets where applicable.
