# Changelog - Air

Air is our background daemon sidecar - the Rust process Mountain spawns
for updates, downloads, cryptographic signing, and the long-running
indexing work we don't want blocking the workbench. This file records
what we built in our voice, version by version. Format adapted from
[Keep a Changelog](https://keepachangelog.com/).

## [v2.1] - Full Workbench Lift

We brought Air's diagnostic surface in line with the rest of the
sidecar fleet so its log output reads the same as Mountain's and
Cocoon's during a workbench boot.

### Added

- **Tag-filtered DevLog system** in `Source/DevLog.rs` (~295 lines),
  threaded through 44 files (1,208 insertions, 1,615 deletions). Air
  now honours the same `LAND_DEV_LOG=short|long` knob the rest of the
  fleet does, and tags its lines with the subsystem so we can grep
  for `[updater]`, `[indexer]`, `[downloader]`, etc. without false
  positives from Mountain.

### Changed

- **PascalCase identifier sweep** across `Binary/Monitor`,
  `Binary/Shutdown`, and every source file in the crate. Brought Air
  fully in line with our project-wide naming rule.
- **Obsolete TODOs cleared** (12 changes). The ones that survived now
  reference active work.
- **Consistent formatting pass** across 14 files (275 changes) so
  `cargo +nightly fmt` produces no diff.

## [v2.0] - Editor Launch (Massive Buildout)

The buildout cycle. **73 Rust modules** landed in this window - Air
went from a placeholder to a real sidecar with a complete updater,
downloader, indexer, and authentication surface.

### Added

- **TODO closure**: a single commit (ca1542c) closed **35 subsystems**
  that had been sitting as stubs - 529 insertions across 35 files.
  Each stub became a real implementation tied to a real call site in
  Mountain or the workbench.
- **DNS resolver layer** in `Source/HTTP/client.rs` (160 lines) with a
  local resolver for security hardening - we don't want our updater
  resolving against an attacker-controlled DNS path. Backed by
  `tests/dns_resolver_tests.rs` (367 lines) covering trust boundaries,
  fallback behaviour, and timeout cases.
- **Module inventory** stabilised at 73 modules across these areas:
  - **Core**: `Source/Binary.rs` (66 KB) sidecar entry point,
    `Source/Library.rs` (21 KB) core architecture and module exports,
    `Source/DevLog.rs` (295 lines) structured logging.
  - **`ApplicationState/`** for persistent state.
  - **`Updates/`** with download verification, patch application,
    versioning.
  - **`Downloader/`** - resilient multi-file downloads with
    pause/resume.
  - **`Authentication/`** with cryptographic signing and secure login.
  - **`Indexing/`** (7 modules) covering file scanning, watching, and
    storage queries.
  - **`HTTP/`** - DNS security, local resolver, resilient HTTP client.
  - **`Mountain/`** - the daemon's side of Mountain communication.
  - **`Vine/Server/AirVinegRPCService.rs`** (281 lines) - the gRPC
    handler Mountain calls into.
  - **`Binary/Monitor/`** for health monitoring and state tracking.
  - **`Binary/Shutdown/`** for graceful shutdown coordination.
  - **`Configuration/HotReload.rs`** for config-reload detection.
  - **`Logging/`, `Metrics/`, `Tracing/`** - the observability trio.
  - **`Resilience/`, `Security/`, `Plugins/`** - cross-cutting
    concerns shared across subsystems.

### Changed

- **Field-naming standardisation** to singular form across the crate
  (192 field renames). Brought Air in line with the DTO conventions
  Common locks down for the rest of the fleet.
- **`Cargo.toml`** picked up 12 new dependencies for the DNS/HTTP
  hardening pass.

### Dependencies

`tokio`, `tonic`, `serde`, `serde_json`, `reqwest`, `futures`, our own
`Mist`, `async-trait`, `chrono`, `uuid`, `walkdir`, `ignore`, `rustls`,
`rustls-pemfile`, `tokio-rustls`, `rustls-native-certs`. Cargo
features: `full-services` (auth + updates + downloader + indexing),
`mtls`, `appimage`, `deb`, `rpm`. Platform: `libc` on Unix, `windows
0.62` on Windows.

## [v1.2] - Full-Stack Integration

Initial architecture planning, minimal source activity (permission
resets, `.gitmodules` upkeep). Air sat as a placeholder while the rest
of the stack matured around it - the buildout came in v2.0.
