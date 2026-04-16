# Changelog

All notable changes to Air (Background Daemon) are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/).

## [v2.1] — Q2 2026: Full Workbench Lift

### Added

- Tag-filtered DevLog system (DevLog.rs, 295 lines) across 44 files (1,208
  insertions, 1,615 deletions)
- README expanded (+137 lines, +192 lines em quad formatting)

### Changed

- PascalCase identifier refactoring: Binary/Monitor, Binary/Shutdown, all
  source files
- Obsolete TODOs removed (12 changes)
- Consistent formatting pass (14 files, 275 changes)

## [v2.0] — Q1 2026: Editor Launch Sprint

**73 Rust modules — massive buildout quarter.**

### March 4: TODO Closure (Pivotal Commit ca1542c)

35 subsystems completed in single commit: 529 insertions across 35 files.

### February 27: DNS Resolver

#### Added

- `Source/HTTP/client.rs` (160 lines) — DNS-based security with local resolver
- `tests/dns_resolver_tests.rs` (367 lines) — comprehensive test suite
- 23 files modified, 786 insertions, 144 deletions

### Full Module Inventory (73 modules)

#### Core

- `Source/Binary.rs` (66KB) — sidecar entry point
- `Source/Library.rs` (21KB) — core architecture + module exports
- `Source/DevLog.rs` (295 lines) — structured logging

#### Subsystems

- `Source/ApplicationState/` — persistent state management
- `Source/Updates/` — download verification, patch application, versioning
- `Source/Downloader/` — resilient multi-file with pause/resume
- `Source/Authentication/` — cryptographic signing, secure login
- `Source/Indexing/` (7 modules) — file scanning, watching, storage queries
- `Source/HTTP/client.rs` — DNS security, local resolver, resilient HTTP
- `Source/Mountain/` — Mountain daemon communication
- `Source/Vine/Server/AirVinegRPCService.rs` (281 lines) — gRPC handler
- `Source/Binary/Monitor/` — health monitoring, state tracking
- `Source/Binary/Shutdown/` — graceful shutdown coordination
- `Source/Configuration/HotReload.rs` — config reload detection
- `Source/Logging/`, `Source/Metrics/`, `Source/Tracing/` — observability
- `Source/Resilience/`, `Source/Security/`, `Source/Plugins/` — cross-cutting

### Changed

- Field naming: singular form standardization (192 field renames)
- Cargo.toml: 12 dependency additions (DNS/HTTP crates)

### Dependencies

- tokio, tonic, serde, serde_json, reqwest, futures, Mist, async-trait,
  chrono, uuid, walkdir, ignore, rustls, rustls-pemfile, tokio-rustls,
  rustls-native-certs
- Features: full-services (auth + updates + downloader + indexing), mtls,
  appimage, deb, rpm
- Platform: libc (Unix), windows 0.62 (Windows)

## [v1.2] — Q3 2025: Full Stack Integration

### Added

- Initial architecture planning
- Minimal activity (permission resets, .gitmodules management)
