# Air Configuration Reference

This document lists primary configuration keys used by Air. Configuration is typically TOML or JSON; the project includes a config loader that maps keys into the `Config` struct.

Common configuration sections and keys

- `grpc`:
  - `port` (int) — default `50053`.
  - `bind_address` (string) — default `127.0.0.1`.
  - `tls.enabled` (bool) — enable TLS.
  - `tls.cert_path`, `tls.key_path`, `tls.ca_path`.

- `logging`:
  - `level` — `error`, `warn`, `info`, `debug`, `trace`.
  - `format` — `json` or `plain`.

- `storage`:
  - `state_dir` — default `/var/lib/air`.
  - `cache_dir` — default `/var/cache/air`.

- `updates`:
  - `channel` — `stable`, `beta`, `nightly`.
  - `update_server` — base URL for update metadata.
  - `verify_checksum` — bool; enable SHA256 verification for downloads.

- `downloads`:
  - `concurrency` — number of parallel downloader workers.
  - `retries` — retry count for transient network errors.
  - `timeout_seconds` — per-download timeout.

- `resources`:
  - `memory_limit_mb`, `cpu_limit_percent`, `disk_limit_mb` — resource throttling for heavy tasks.

Example: `examples/config/local.toml`

```toml
[grpc]
port = 50053
bind_address = "127.0.0.1"

[grpc.tls]
enabled = false

[logging]
level = "debug"
format = "plain"

[storage]
state_dir = "./state"
cache_dir = "./cache"

[updates]
channel = "stable"
update_server = "https://updates.land"
verify_checksum = true

[downloads]
concurrency = 4
retries = 3
timeout_seconds = 60

[resources]
memory_limit_mb = 1024
cpu_limit_percent = 50
```

Environment-specific examples

- Production (systemd): put config in `/etc/air/config.toml`, enable TLS, mount `/var/lib/air` and `/etc/air/certs` with appropriate permissions.
- Container: include config at container image `/etc/air/config.toml` or provide via mounted volume. Use read-only mounts for certs.

Troubleshooting

- Cannot bind port: ensure another process is not using `50053` and check `bind_address`.
- TLS errors: confirm `cert_path`/`key_path` are readable by the service user and certificates are valid.
- Persistent state missing after restart: check `state_dir` ownership and permissions.
- Downloads failing: enable `logging.level = "debug"` and inspect downloader logs; verify network/proxy settings.
