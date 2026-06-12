# **Air** 🪁

The native background daemon for the Land editor.

> **VS Code cold-starts slowly because everything initializes fresh each launch. Updates require a full restart that kills open terminals and in-progress work. There is no mechanism to pre-stage work between sessions.**
>
> _"The next version is already downloaded and verified before you decide to update. The main window never blocks waiting for a download."_

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](https://github.com/CodeEditorLand/Air/tree/Current/LICENSE)

**[Rust API Documentation](https://Rust.Documentation.editor.land/Air/)** 📖

---

## Overview

**Air** is the lightweight, persistent daemon that powers the background capabilities of the **Land** Code Editor. While **Mountain** handles the core application logic and UI, **Air** operates as a specialized sidecar process dedicated to heavy lifting, network operations, and system maintenance. It ensures that the main editor remains responsive by offloading resource-intensive tasks such as updates, large downloads, cryptographic signing, and file indexing.

**Air** is engineered to:

1. **Serve as the Persistent Background Daemon:** Run as a standalone process alongside **Mountain**, surviving window closures and maintaining background services across sessions.
2. **Own the Update Lifecycle:** Take full ownership of downloading, verifying, and applying patches for **Land** without user interruption or restart prompts.
3. **Offload Heavy Network Operations:** Act as the traffic manager for large downloads (extensions, language servers, dependencies) with resilient resume-capable transfers.
4. **Isolate Security-Critical Operations:** Manage cryptographic signing, secure credential storage, and authentication token lifecycle, keeping sensitive logic isolated from the main application view.
5. **Maintain the File Index:** Build and persist a comprehensive file index with symbol extraction and fast fuzzy search across the entire workspace.

## Architecture

```mermaid
graph LR
    classDef mountain fill:#f0d0ff,stroke:#9b59b6,stroke-width:2px,color:#2c0050;
    classDef air      fill:#e0f4ff,stroke:#2471a3,stroke-width:2px,color:#001040;
    classDef external fill:#ebebeb,stroke:#888,stroke-width:1px,stroke-dasharray:5 5,color:#333;
    classDef infra    fill:#fff3c0,stroke:#f39c12,stroke-width:1px,stroke-dasharray:5 5,color:#5a3e00;

    subgraph MOUNTAIN["Mountain ⛰️ - Main Application"]
        MountainIPC["Mountain gRPC client\ndelegates heavy tasks"]:::mountain
    end

    subgraph AIR["Air 🪁 - Persistent Background Daemon (::1:50053)"]
        direction TB
        subgraph COMM["Vine/ - gRPC Transport"]
            VineServer["Vine/Server/ - gRPC server\n(Generated/ prost bindings)"]:::air
            MountainClient["Mountain gRPC client\n(Air → Mountain callbacks)"]:::air
        end
        subgraph CORE["Core Services"]
            Updates["Updates/ - version check\ndownload · verify · staged install · rollback"]:::air
            Downloader["Downloader/ - parallel chunks\nrate-limit · resume · retry"]:::air
            Auth["Authentication/ - token mgmt\nAEAD encrypt · key rotation"]:::air
            Indexing["Indexing/ - file index\nsymbol extract · FS watch · search"]:::air
        end
        subgraph INFRA["Infrastructure"]
            Health["HealthCheck/ - Alive/Responsive/Functional\nauto-recovery"]:::infra
            Resilience["Resilience/ - retry backoff\ncircuit breaker · bulkhead"]:::infra
            Metrics["Metrics/ - Prometheus-compatible\nlatency · success rate"]:::infra
            Security["Security/ - AES-GCM\nchecksum · audit"]:::infra
            Daemon["Daemon/ - PID lock\nsingleton enforce"]:::air
        end

        VineServer --> Updates
        VineServer --> Downloader
        VineServer --> Auth
        VineServer --> Indexing
        Updates --> Resilience
        Downloader --> Resilience
    end

    subgraph EXTERNAL["External ☁️"]
        UpdateSrv["Update servers / extension registry"]:::external
    end

    MountainIPC -- gRPC :50053 --> VineServer
    MountainClient -- progress events --> MountainIPC
    Updates -- fetches --> UpdateSrv
    Downloader -- downloads --> UpdateSrv
```

### Core Architecture Principles

| Principle | Description | Key Components |
| :--------- | :---------- | :------------- |
| **Sidecar Isolation** | Run as a standalone daemon process, surviving independently of the main window lifecycle for persistent background operations. | `Daemon/`, `Binary/`, PID locking |
| **gRPC IPC Boundary** | Use **Vine** (`tonic`-based gRPC) for all communication with **Mountain**, ensuring a high-performance and well-defined API. | `Vine/`, `Air.proto`, generated prost bindings |
| **Service Modularity** | Each capability (updates, downloads, auth, indexing) lives in its own module with independent startup and health monitoring. | `Updates/`, `Downloader/`, `Authentication/`, `Indexing/` |
| **Resilience by Default** | Wrap all network operations in retry-with-backoff, circuit breakers, bulkheads, and timeouts via the shared `Resilience/` lib. | `Resilience/`, `HealthCheck/` |
| **Declarative Configuration** | Load TOML config with schema validation, environment overrides, and hot reload without service interruption. | `Configuration/`, `Initialize/` |
| **Observable Operations** | Emit structured JSON logs, distributed traces, and Prometheus metrics for every delegated task. | `Logging/`, `Tracing/`, `Metrics/` |
| **Secure Credential Handling** | Never expose raw secrets; store credentials with AEAD encryption (`ring`), enforce key rotation, and audit all access. | `Security/`, `Authentication/`, `zeroize` |

## Key Components

| Component | Path | Description |
| --------- | ---- | ----------- |
| Binary Entry Point | `Source/Binary.rs` | Binary entry point for the Air daemon. |
| Library Entry | `Source/Library.rs` | Module declarations and crate-level exports. |
| Daemon Lifecycle | `Source/Binary/` | Daemon process lifecycle (startup, shutdown, monitoring). |
| Singleton Enforcer | `Source/Daemon/` | Singleton enforcement, PID locking, platform-native integration. |
| Initialization | `Source/Initialize/` | Configuration, port binding, gRPC server construction, per-service startup. |
| CLI | `Source/CLI/` | Command-line interface for daemon interaction and diagnostics. |
| gRPC Protocol | `Source/Vine/` | gRPC protocol implementation (generated proto, server, errors). |
| Application State | `Source/ApplicationState/` | Central coordination (connections, service states, telemetry, resources). |
| Configuration | `Source/Configuration/` | TOML config loading with schema validation, env overrides, hot reload. |
| Logging | `Source/DevLog.rs` | Developer-facing logging and trace ID generation. |
| Updates | `Source/Updates/` | Version checking, download, verification, staged install, rollback. |
| Downloader | `Source/Downloader/` | Parallel downloads, chunk transfers, rate limiting, resume capability. |
| Authentication | `Source/Authentication/` | Token management, credential storage, AEAD encryption, key rotation. |
| Indexing | `Source/Indexing/` | File index, symbol extraction, scanning, persistent storage, FS watch. |
| Health Check | `Source/HealthCheck/` | Multi-level health monitoring (alive, responsive, functional). |
| Logging System | `Source/Logging/` | Structured JSON logging with trace ID propagation and rotation. |
| Metrics | `Source/Metrics/` | Prometheus-compatible metrics (latency, success rate, resource usage). |
| Resilience | `Source/Resilience/` | Retry with backoff, circuit breaker, bulkhead, timeout management. |
| Security | `Source/Security/` | Checksum verification, AES-GCM credential storage, audit subsystem. |
| Tracing | `Source/Tracing/` | Distributed tracing with sampling, span events, context propagation. |
| HTTP Client | `Source/HTTP/` | Secure HTTP client with custom DNS, TLS, timeout management. |

## In the Land Project

**Air** is the persistent background daemon for the Land ecosystem. It communicates with **Mountain** via **Vine** (gRPC) on port `[::1]:50053` and uses **Mist** for DNS isolation on its HTTP client.

| Role | Details |
| :--- | :------ |
| **Daemon Process** | Persistent executable that runs independently of the main window, even after the window closes. |
| **Server Host** | Hosts a local gRPC server on `[::1]:50053` to accept commands from **Mountain**. |
| **Update Delegate** | Sole authority for modifying installation files of the parent application. |
| **Signer** | Handles cryptographic signing of artifacts and secure token storage for user login. |
| **Traffic Manager** | Proxy/downloader that keeps large network operations off the main renderer process. |
| **File Indexer** | Maintains a persistent file index with symbol extraction and fast search across the workspace. |
| **Health Monitor** | Periodically checks all service health with automatic recovery and degradation tracking. |

### Port Allocation

| Process | Port | Protocol | Purpose |
| :------ | :--- | :------- | :------ |
| **Air** | `50053` | **Vine**/`Air.proto` (gRPC) | Daemon services — updates, downloads |
| **Cocoon** | `50052` | `Vine.proto` (gRPC) | VS Code extension hosting |

**Air** is part of the networking/IPC connectivity stack alongside **Mist** 🌫️ (DNS isolation) and **Vine** 🍇 (gRPC protocol layer).

Typical usage flow:
1. **Spawn:** **Mountain** detects if **Air** is running. If not, it spawns the binary.
2. **Connect:** **Mountain** establishes a **Vine** (gRPC) connection to **Air**'s local port `[::1]:50053`.
3. **Delegate:** When a user requests an update or large download, **Mountain** sends a command to **Air** and immediately returns control to the user.
4. **Monitor:** **Air** emits progress events back to **Mountain** to update the UI status bars.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Protocol Buffer compiler (included via `tonic-build` build dependency)

### Build

```bash
cd Element/Air
cargo build --release
```

### Run

```bash
# Run with default settings
./Target/release/Air

# Or via cargo
cargo run --bin Air
```

### Key Dependencies

| Crate / Package | Purpose |
| :-------------- | :------ |
| `tonic` / `prost` | gRPC server and Protocol Buffer code generation |
| **Vine** | Local path dependency — generated `Air.proto` gRPC contracts |
| `Common` | Local path dependency — shared types and abstractions |
| **Mist** | Local path dependency — DNS isolation for HTTP client |
| `reqwest` / `rustls` | HTTPS downloads with TLS certificate verification |
| `tokio` | Async runtime for concurrent I/O and task scheduling |
| `notify` / `ignore` | File system event watching for real-time index updates |
| `ring` / `zeroize` | Cryptographic signing and secure credential storage |
| `tracing` | Structured JSON logging with span propagation |
| `config` / `toml` | Configuration file loading with hot-reload support |
| `sysinfo` / `systemstat` | System resource monitoring and health checks |
| `walkdir` / `ignore` | Recursive directory traversal for file indexing |

## API Reference

- [Rust API Documentation](https://Rust.Documentation.editor.land/Air/) 📖
- [Deep Dive](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/DeepDive.md) — Detailed startup sequence, gRPC routing, and data flow

## Related Documentation

- [Architecture Overview](../Documentation/GitHub/Architecture.md) — Internal module structure
- [Deep Dive](../Documentation/GitHub/DeepDive.md) — In-depth technical details
- [Land Documentation](../../Documentation/GitHub/README.md) — Complete documentation index
- **Mist** 🌫️ — DNS isolation for the private network — [GitHub](https://github.com/CodeEditorLand/Mist)
- **Vine** 🍇 — gRPC protocol layer — [GitHub](https://github.com/CodeEditorLand/Vine)
- **Mountain** ⛰️ — Main application process — [GitHub](https://github.com/CodeEditorLand/Mountain)
- **Common** — Shared types and abstractions — [GitHub](https://github.com/CodeEditorLand/Common)
- **Echo** — [GitHub](https://github.com/CodeEditorLand/Echo)

---

## Funding

This project is funded through [NGI0 Commons Fund](https://NLnet.NL/commonsfund), a fund established by [NLnet](https://NLnet.NL) with financial support from the European Commission's Next Generation Internet program, under grant agreement No 101135429.

The project is operated by PlayForm, based in Sofia, Bulgaria. PlayForm acts as the open-source steward for Code Editor Land under the NGI0 Commons Fund grant.

| | |
| --- | --- |
| [![Land](https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/Dual/Land.svg)](https://Editor.Land) | [![PlayForm](https://raw.githubusercontent.com/PlayForm/Asset/refs/heads/Current/Logo/PlayForm.svg)](https://PlayForm.Cloud) |
| [![NLnet](https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/NLnet.svg)](https://NLnet.NL) | [![NGI0](https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/NGI0.svg)](https://NLnet.NL/commonsfund) |

---

**Project Maintainers**: Source Open (Source/Open@editor.land) | [GitHub Repository](https://github.com/CodeEditorLand/Air) | [Report an Issue](https://github.com/CodeEditorLand/Air/issues) | [Security Policy](https://github.com/CodeEditorLand/Air/security/policy)
